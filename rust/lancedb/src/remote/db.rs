// Copyright 2024 LanceDB Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

use arrow_array::RecordBatchReader;
use async_trait::async_trait;
use http::StatusCode;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use tokio::task::spawn_blocking;

use crate::connection::{
    ConnectionInternal, CreateTableBuilder, NoData, OpenTableBuilder, TableNamesBuilder,
};
use crate::embeddings::EmbeddingRegistry;
use crate::error::Result;
use crate::Table;

use super::client::{HttpSend, RestfulLanceDbClient, Sender};
use super::table::RemoteTable;
use super::util::batches_to_ipc_bytes;
use super::ARROW_STREAM_CONTENT_TYPE;

#[derive(Deserialize)]
struct ListTablesResponse {
    tables: Vec<String>,
}

#[derive(Debug)]
pub struct RemoteDatabase<S: HttpSend = Sender> {
    client: RestfulLanceDbClient<S>,
}

impl RemoteDatabase {
    pub fn try_new(
        uri: &str,
        api_key: &str,
        region: &str,
        host_override: Option<String>,
    ) -> Result<Self> {
        let client = RestfulLanceDbClient::try_new(uri, api_key, region, host_override)?;
        Ok(Self { client })
    }
}

#[cfg(all(test, feature = "remote"))]
mod test_utils {
    use super::*;
    use crate::remote::client::test_utils::client_with_handler;
    use crate::remote::client::test_utils::MockSender;

    impl RemoteDatabase<MockSender> {
        pub fn new_mock<F, T>(handler: F) -> Self
        where
            F: Fn(reqwest::Request) -> http::Response<T> + Send + Sync + 'static,
            T: Into<reqwest::Body>,
        {
            let client = client_with_handler(handler);
            Self { client }
        }
    }
}

impl<S: HttpSend> std::fmt::Display for RemoteDatabase<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RemoteDatabase(host={})", self.client.host())
    }
}

#[async_trait]
impl<S: HttpSend> ConnectionInternal for RemoteDatabase<S> {
    async fn table_names(&self, options: TableNamesBuilder) -> Result<Vec<String>> {
        let mut req = self.client.get("/v1/table/");
        if let Some(limit) = options.limit {
            req = req.query(&[("limit", limit)]);
        }
        if let Some(start_after) = options.start_after {
            req = req.query(&[("page_token", start_after)]);
        }
        let rsp = self.client.send(req).await?;
        let rsp = self.client.check_response(rsp).await?;
        Ok(rsp.json::<ListTablesResponse>().await?.tables)
    }

    async fn do_create_table(
        &self,
        options: CreateTableBuilder<false, NoData>,
        data: Box<dyn RecordBatchReader + Send>,
    ) -> Result<Table> {
        // TODO: https://github.com/lancedb/lancedb/issues/1026
        // We should accept data from an async source.  In the meantime, spawn this as blocking
        // to make sure we don't block the tokio runtime if the source is slow.
        let data_buffer = spawn_blocking(move || batches_to_ipc_bytes(data))
            .await
            .unwrap()?;

        let req = self
            .client
            .post(&format!("/v1/table/{}/create/", options.name))
            .body(data_buffer)
            .header(CONTENT_TYPE, ARROW_STREAM_CONTENT_TYPE)
            // This is currently expected by LanceDb cloud but will be removed soon.
            .header("x-request-id", "na");
        let rsp = self.client.send(req).await?;

        if rsp.status() == StatusCode::BAD_REQUEST {
            let body = rsp.text().await?;
            if body.contains("already exists") {
                return Err(crate::Error::TableAlreadyExists { name: options.name });
            } else {
                return Err(crate::Error::InvalidInput { message: body });
            }
        }

        self.client.check_response(rsp).await?;

        Ok(Table::new(Arc::new(RemoteTable::new(
            self.client.clone(),
            options.name,
        ))))
    }

    async fn do_open_table(&self, options: OpenTableBuilder) -> Result<Table> {
        // We describe the table to confirm it exists before moving on.
        // TODO: a TTL cache of table existence
        let req = self
            .client
            .get(&format!("/v1/table/{}/describe/", options.name));
        let resp = self.client.send(req).await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Err(crate::Error::TableNotFound { name: options.name });
        }
        self.client.check_response(resp).await?;
        Ok(Table::new(Arc::new(RemoteTable::new(
            self.client.clone(),
            options.name,
        ))))
    }

    async fn drop_table(&self, name: &str) -> Result<()> {
        let req = self.client.post(&format!("/v1/table/{}/drop/", name));
        let resp = self.client.send(req).await?;
        self.client.check_response(resp).await?;
        Ok(())
    }

    async fn drop_db(&self) -> Result<()> {
        Err(crate::Error::NotSupported {
            message: "Dropping databases is not supported in the remote API".to_string(),
        })
    }

    fn embedding_registry(&self) -> &dyn EmbeddingRegistry {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow_array::{Int32Array, RecordBatch, RecordBatchIterator};
    use arrow_schema::{DataType, Field, Schema};

    use crate::{remote::db::ARROW_STREAM_CONTENT_TYPE, Connection};

    #[tokio::test]
    async fn test_table_names() {
        let conn = Connection::new_with_handler(|request| {
            assert_eq!(request.method(), &reqwest::Method::GET);
            assert_eq!(request.url().path(), "/v1/table/");
            assert_eq!(request.url().query(), None);

            http::Response::builder()
                .status(200)
                .body(r#"{"tables": ["table1", "table2"]}"#)
                .unwrap()
        });
        let names = conn.table_names().execute().await.unwrap();
        assert_eq!(names, vec!["table1", "table2"]);
    }

    #[tokio::test]
    async fn test_table_names_pagination() {
        let conn = Connection::new_with_handler(|request| {
            assert_eq!(request.method(), &reqwest::Method::GET);
            assert_eq!(request.url().path(), "/v1/table/");
            assert!(request.url().query().unwrap().contains("limit=2"));
            assert!(request.url().query().unwrap().contains("page_token=table2"));

            http::Response::builder()
                .status(200)
                .body(r#"{"tables": ["table3", "table4"], "page_token": "token"}"#)
                .unwrap()
        });
        let names = conn
            .table_names()
            .start_after("table2")
            .limit(2)
            .execute()
            .await
            .unwrap();
        assert_eq!(names, vec!["table3", "table4"]);
    }

    #[tokio::test]
    async fn test_open_table() {
        let conn = Connection::new_with_handler(|request| {
            assert_eq!(request.method(), &reqwest::Method::GET);
            assert_eq!(request.url().path(), "/v1/table/table1/describe/");
            assert_eq!(request.url().query(), None);

            http::Response::builder()
                .status(200)
                .body(r#"{"table": "table1"}"#)
                .unwrap()
        });
        let table = conn.open_table("table1").execute().await.unwrap();
        assert_eq!(table.name(), "table1");

        // Storage options should be ignored.
        let table = conn
            .open_table("table1")
            .storage_option("key", "value")
            .execute()
            .await
            .unwrap();
        assert_eq!(table.name(), "table1");
    }

    #[tokio::test]
    async fn test_open_table_not_found() {
        let conn = Connection::new_with_handler(|_| {
            http::Response::builder()
                .status(404)
                .body("table not found")
                .unwrap()
        });
        let result = conn.open_table("table1").execute().await;
        assert!(result.is_err());
        assert!(matches!(result, Err(crate::Error::TableNotFound { .. })));
    }

    #[tokio::test]
    async fn test_create_table() {
        let conn = Connection::new_with_handler(|request| {
            assert_eq!(request.method(), &reqwest::Method::POST);
            assert_eq!(request.url().path(), "/v1/table/table1/create/");
            assert_eq!(
                request
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .unwrap(),
                ARROW_STREAM_CONTENT_TYPE.as_bytes()
            );

            http::Response::builder().status(200).body("").unwrap()
        });
        let data = RecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)])),
            vec![Arc::new(Int32Array::from(vec![1, 2, 3]))],
        )
        .unwrap();
        let reader = RecordBatchIterator::new([Ok(data.clone())], data.schema());
        let table = conn.create_table("table1", reader).execute().await.unwrap();
        assert_eq!(table.name(), "table1");
    }

    #[tokio::test]
    async fn test_create_table_already_exists() {
        let conn = Connection::new_with_handler(|_| {
            http::Response::builder()
                .status(400)
                .body("table table1 already exists")
                .unwrap()
        });
        let data = RecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)])),
            vec![Arc::new(Int32Array::from(vec![1, 2, 3]))],
        )
        .unwrap();
        let reader = RecordBatchIterator::new([Ok(data.clone())], data.schema());
        let result = conn.create_table("table1", reader).execute().await;
        assert!(result.is_err());
        assert!(
            matches!(result, Err(crate::Error::TableAlreadyExists { name }) if name == "table1")
        );
    }

    #[tokio::test]
    async fn test_create_table_empty() {
        let conn = Connection::new_with_handler(|request| {
            assert_eq!(request.method(), &reqwest::Method::POST);
            assert_eq!(request.url().path(), "/v1/table/table1/create/");
            assert_eq!(
                request
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .unwrap(),
                ARROW_STREAM_CONTENT_TYPE.as_bytes()
            );

            http::Response::builder().status(200).body("").unwrap()
        });
        let schema = Arc::new(Schema::new(vec![Field::new("a", DataType::Int32, false)]));
        conn.create_empty_table("table1", schema)
            .execute()
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_drop_table() {
        let conn = Connection::new_with_handler(|request| {
            assert_eq!(request.method(), &reqwest::Method::POST);
            assert_eq!(request.url().path(), "/v1/table/table1/drop/");
            assert_eq!(request.url().query(), None);
            assert!(request.body().is_none());

            http::Response::builder().status(200).body("").unwrap()
        });
        conn.drop_table("table1").await.unwrap();
        // NOTE: the API will return 200 even if the table does not exist. So we shouldn't expect 404.
    }
}
