from typing import Dict, List, Optional, Tuple, Any, Union, Literal

import pyarrow as pa

from .index import BTree, IvfFlat, IvfPq, Bitmap, LabelList, HnswPq, HnswSq, FTS
from .remote import ClientConfig

class Connection(object):
    uri: str
    async def table_names(
        self, start_after: Optional[str], limit: Optional[int]
    ) -> list[str]: ...
    async def create_table(
        self,
        name: str,
        mode: str,
        data: pa.RecordBatchReader,
        storage_options: Optional[Dict[str, str]] = None,
    ) -> Table: ...
    async def create_empty_table(
        self,
        name: str,
        mode: str,
        schema: pa.Schema,
        storage_options: Optional[Dict[str, str]] = None,
    ) -> Table: ...
    async def rename_table(self, old_name: str, new_name: str) -> None: ...
    async def drop_table(self, name: str) -> None: ...

class Table:
    def name(self) -> str: ...
    def __repr__(self) -> str: ...
    def is_open(self) -> bool: ...
    def close(self) -> None: ...
    async def schema(self) -> pa.Schema: ...
    async def add(
        self, data: pa.RecordBatchReader, mode: Literal["append", "overwrite"]
    ) -> None: ...
    async def update(self, updates: Dict[str, str], where: Optional[str]) -> None: ...
    async def count_rows(self, filter: Optional[str]) -> int: ...
    async def create_index(
        self,
        column: str,
        index: Union[IvfFlat, IvfPq, HnswPq, HnswSq, BTree, Bitmap, LabelList, FTS],
        replace: Optional[bool],
    ): ...
    async def list_versions(self) -> List[Dict[str, Any]]: ...
    async def version(self) -> int: ...
    async def checkout(self, version: int): ...
    async def checkout_latest(self): ...
    async def restore(self, version: Optional[int] = None): ...
    async def list_indices(self) -> list[IndexConfig]: ...
    async def delete(self, filter: str): ...
    async def add_columns(self, columns: list[tuple[str, str]]) -> None: ...
    async def add_columns_with_schema(self, schema: pa.Schema) -> None: ...
    async def alter_columns(self, columns: list[dict[str, Any]]) -> None: ...
    async def optimize(
        self,
        *,
        cleanup_since_ms: Optional[int] = None,
        delete_unverified: Optional[bool] = None,
    ) -> OptimizeStats: ...
    def query(self) -> Query: ...
    def vector_search(self) -> VectorQuery: ...

class IndexConfig:
    index_type: str
    columns: List[str]

async def connect(
    uri: str,
    api_key: Optional[str],
    region: Optional[str],
    host_override: Optional[str],
    read_consistency_interval: Optional[float],
    client_config: Optional[Union[ClientConfig, Dict[str, Any]]],
    storage_options: Optional[Dict[str, str]],
) -> Connection: ...

class RecordBatchStream:
    @property
    def schema(self) -> pa.Schema: ...
    def __aiter__(self) -> "RecordBatchStream": ...
    async def __anext__(self) -> pa.RecordBatch: ...

class Query:
    def where(self, filter: str): ...
    def select(self, columns: Tuple[str, str]): ...
    def select_columns(self, columns: List[str]): ...
    def limit(self, limit: int): ...
    def offset(self, offset: int): ...
    def fast_search(self): ...
    def with_row_id(self): ...
    def postfilter(self): ...
    def nearest_to(self, query_vec: pa.Array) -> VectorQuery: ...
    def nearest_to_text(self, query: dict) -> FTSQuery: ...
    async def execute(self, max_batch_length: Optional[int]) -> RecordBatchStream: ...
    async def explain_plan(self, verbose: Optional[bool]) -> str: ...
    async def analyze_plan(self) -> str: ...
    def to_query_request(self) -> PyQueryRequest: ...

class FTSQuery:
    def where(self, filter: str): ...
    def select(self, columns: List[str]): ...
    def limit(self, limit: int): ...
    def offset(self, offset: int): ...
    def fast_search(self): ...
    def with_row_id(self): ...
    def postfilter(self): ...
    def get_query(self) -> str: ...
    def add_query_vector(self, query_vec: pa.Array) -> None: ...
    def nearest_to(self, query_vec: pa.Array) -> HybridQuery: ...
    async def execute(self, max_batch_length: Optional[int]) -> RecordBatchStream: ...
    def to_query_request(self) -> PyQueryRequest: ...

class VectorQuery:
    async def execute(self) -> RecordBatchStream: ...
    def where(self, filter: str): ...
    def select(self, columns: List[str]): ...
    def select_with_projection(self, columns: Tuple[str, str]): ...
    def limit(self, limit: int): ...
    def offset(self, offset: int): ...
    def column(self, column: str): ...
    def distance_type(self, distance_type: str): ...
    def postfilter(self): ...
    def refine_factor(self, refine_factor: int): ...
    def nprobes(self, nprobes: int): ...
    def bypass_vector_index(self): ...
    def nearest_to_text(self, query: dict) -> HybridQuery: ...
    def to_query_request(self) -> PyQueryRequest: ...

class HybridQuery:
    def where(self, filter: str): ...
    def select(self, columns: List[str]): ...
    def limit(self, limit: int): ...
    def offset(self, offset: int): ...
    def fast_search(self): ...
    def with_row_id(self): ...
    def postfilter(self): ...
    def distance_type(self, distance_type: str): ...
    def refine_factor(self, refine_factor: int): ...
    def nprobes(self, nprobes: int): ...
    def bypass_vector_index(self): ...
    def to_vector_query(self) -> VectorQuery: ...
    def to_fts_query(self) -> FTSQuery: ...
    def get_limit(self) -> int: ...
    def get_with_row_id(self) -> bool: ...
    def to_query_request(self) -> PyQueryRequest: ...

class PyFullTextSearchQuery:
    columns: Optional[List[str]]
    query: str
    limit: Optional[int]
    wand_factor: Optional[float]

class PyQueryRequest:
    limit: Optional[int]
    offset: Optional[int]
    filter: Optional[Union[str, bytes]]
    full_text_search: Optional[PyFullTextSearchQuery]
    select: Optional[Union[str, List[str]]]
    fast_search: Optional[bool]
    with_row_id: Optional[bool]
    column: Optional[str]
    query_vector: Optional[List[pa.Array]]
    nprobes: Optional[int]
    lower_bound: Optional[float]
    upper_bound: Optional[float]
    ef: Optional[int]
    refine_factor: Optional[int]
    distance_type: Optional[str]
    bypass_vector_index: Optional[bool]
    postfilter: Optional[bool]
    norm: Optional[str]

class CompactionStats:
    fragments_removed: int
    fragments_added: int
    files_removed: int
    files_added: int

class CleanupStats:
    bytes_removed: int
    old_versions: int

class RemovalStats:
    bytes_removed: int
    old_versions_removed: int

class OptimizeStats:
    compaction: CompactionStats
    prune: RemovalStats
