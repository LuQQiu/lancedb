// Copyright 2024 Lance Developers.
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

use jni::objects::{JObject, JString, JValue};
use jni::JNIEnv;
use lance::dataset;
use lancedb::connection::{self, connect, Connection};

use crate::ffi::JNIEnvExt;
use crate::{Result, Error, RT};
use crate::traits::IntoJava;

pub const NATIVE_CONNECTION: &str = "nativeConnectHandle";

pub struct BlockingConnection {
    pub(crate) inner: Connection,
}

impl BlockingConnection {
    pub fn create(dataset_uri: &str) -> Result<Self> {
        let inner = RT.block_on(connect(dataset_uri).execute())?;
        Ok(Self { inner })
    }

    pub fn table_names(&self) -> Result<Vec<String>> {
        Ok(RT.block_on(self.inner.table_names().execute())?)
    }
}

impl IntoJava for BlockingConnection {
    fn into_java<'a>(self, env: &mut JNIEnv<'a>) -> JObject<'a> {
        attach_native_connection(env, self)
    }
}

fn attach_native_connection<'local>(
    env: &mut JNIEnv<'local>,
    connection: BlockingConnection,
) -> JObject<'local> {
    let j_connection = create_java_connection_object(env);
    // This block sets a native Rust object (connection) as a field in the Java object (j_connection).
    // Caution: This creates a potential for memory leaks. The Rust object (connection) is not
    // automatically garbage-collected by Java, and its memory will not be freed unless
    // explicitly handled.
    //
    // To prevent memory leaks, ensure the following:
    // 1. The Java object (`j_connection`) should implement the `java.io.Closeable` interface.
    // 2. Users of this Java object should be instructed to always use it within a try-with-resources
    //    statement (or manually call the `close()` method) to ensure that `self.close()` is invoked.
    match unsafe { env.set_rust_field(&j_connection, NATIVE_CONNECTION, connection) } {
        Ok(_) => j_connection,
        Err(err) => {
            env.throw_new(
                "java/lang/RuntimeException",
                format!(
                    "Failed to set native handle for lancedb connection: {}",
                    err
                ),
            )
            .expect("Error throwing exception");
            JObject::null()
        }
    }
}

fn create_java_connection_object<'a>(env: &mut JNIEnv<'a>) -> JObject<'a> {
    env.new_object("com/lancedb/lancedb/Connection", "()V", &[])
        .expect("Failed to create Java Lancedb Connection instance")
}

#[no_mangle]
pub extern "system" fn Java_com_lancedb_lancedb_Connection_releaseNativeConnection(
    mut env: JNIEnv,
    j_connection: JObject,
) {
    let _: BlockingConnection = unsafe {
        env.take_rust_field(j_connection, NATIVE_CONNECTION)
            .expect("Failed to take native Lancedb connection handle")
    };
}

#[no_mangle]
pub extern "system" fn Java_com_lancedb_lancedb_Connection_create<'local>(
    mut env: JNIEnv<'local>,
    _obj: JObject,
    dataset_uri_object: JString,
) -> JObject<'local> {
    let dataset_uri: String = ok_or_throw!(env, env.get_string(&dataset_uri_object)).into();
    let blocking_connection = ok_or_throw!(env, BlockingConnection::create(&dataset_uri));
    blocking_connection.into_java(&mut env)
}

#[no_mangle]
pub extern "system" fn Java_com_lancedb_lancedb_Connection_tableNames<'local>(
    mut env: JNIEnv<'local>,
    _obj: JObject,
    j_connection: JObject,
)  {
    let connection_res = unsafe { env.get_rust_field::<_, _, BlockingConnection>(j_connection, NATIVE_CONNECTION) };
    //let connection = ok_or_throw_without_return!(env, connection_res);
    //let table_names = ok_or_throw_without_return!(env, table_names_result);
}

// #[no_mangle]
// pub extern "system" fn Java_com_lancedb_lancedb_Connection_tableNames2<'local>(
//     mut env: JNIEnv<'local>,
//     _obj: JObject,
//     j_connection: JObject,
// ) -> JObject<'local> {
//     let table_names_result = {
//         let connection = unsafe { env.get_rust_field::<_, _, BlockingConnection>(j_connection, NATIVE_CONNECTION) }
//         .expect("Connection handle not set");
//         connection.table_names()
//     };
//     let table_names = ok_or_throw!(env, table_names_result);

//     let list_class = env.find_class("java/util/ArrayList").expect("msg");
//     let list_obj = env.alloc_object(list_class).expect("ms");
//     env.call_method(&list_obj, "<init>", "()V", &[]).expect("msg");
//     for item in table_names {
//         let item_jobj = JObject::from(env.new_string(item).expect("msg"));
//         let item_gen = JValue::Object(&item_jobj);
//         env.call_method(&list_obj, "add", "(Ljava/lang/Object;)Z", &[item_gen]).expect("msg");
//     };
//     list_obj
// }
