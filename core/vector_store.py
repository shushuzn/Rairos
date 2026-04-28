"""Zilliz Cloud vector store for persistent code embeddings."""
from __future__ import annotations

import os
from typing import List, Optional, Dict, Any
from dataclasses import dataclass

from pymilvus import MilvusClient, DataType


@dataclass
class SearchResult:
    """A search result with score and metadata."""
    id: str
    score: float
    content: str
    file: str
    line: int


class ZillizStore:
    """
    Zilliz Cloud vector store for code embeddings.

    Provides persistent storage and similarity search for code chunks.
    """

    COLLECTION_NAME = "code_chunks"

    def __init__(self, uri: str, token: str, dim: int = 768):
        """Initialize Zilliz client.

        Args:
            uri: Zilliz Cloud URI (e.g., https://xxx.zillizcloud.com:443)
            token: Zilliz API key
            dim: Embedding dimension (nomic-embed-text = 768)
        """
        self.client = MilvusClient(uri=uri, token=token)
        self.dim = dim
        self._ensure_collection()

    def _ensure_collection(self) -> None:
        """Create collection if not exists."""
        if self.client.has_collection(self.COLLECTION_NAME):
            return

        schema = MilvusClient.create_schema(
            auto_id=False,
            enable_dynamic_field=True,
        )

        schema.add_field(
            field_name="id",
            datatype=DataType.VARCHAR,
            max_length=512,
            is_primary=True,
        )
        schema.add_field(
            field_name="vector",
            datatype=DataType.FLOAT_VECTOR,
            dim=self.dim,
        )
        schema.add_field(
            field_name="content",
            datatype=DataType.VARCHAR,
            max_length=4096,
        )
        schema.add_field(
            field_name="file",
            datatype=DataType.VARCHAR,
            max_length=256,
        )
        schema.add_field(
            field_name="line",
            datatype=DataType.INT32,
        )

        index_params = self.client.prepare_index_params()
        index_params.add_index(
            field_name="vector",
            index_type="AUTOINDEX",
            metric_type="COSINE",
        )

        self.client.create_collection(
            collection_name=self.COLLECTION_NAME,
            schema=schema,
            index_params=index_params,
        )
        self.client.load_collection(self.COLLECTION_NAME)

    def upsert(
        self,
        ids: List[str],
        vectors: List[List[float]],
        contents: List[str],
        files: List[str],
        lines: List[int],
    ) -> None:
        """Insert or update embeddings.

        Args:
            ids: Unique chunk IDs
            vectors: Embedding vectors
            contents: Code content
            files: File paths
            lines: Line numbers
        """
        data = [
            {
                "id": id_,
                "vector": vec,
                "content": content,
                "file": file_,
                "line": line,
            }
            for id_, vec, content, file_, line in zip(ids, vectors, contents, files, lines)
        ]

        self.client.upsert(
            collection_name=self.COLLECTION_NAME,
            data=data,
        )

    def search(
        self,
        query_vector: List[float],
        limit: int = 10,
        filter_expr: Optional[str] = None,
    ) -> List[SearchResult]:
        """Search for similar vectors.

        Args:
            query_vector: Query embedding
            limit: Max results
            filter_expr: Optional filter (e.g., 'file like "%test%"')

        Returns:
            List of SearchResult sorted by similarity
        """
        results = self.client.search(
            collection_name=self.COLLECTION_NAME,
            data=[query_vector],
            limit=limit,
            output_fields=["id", "content", "file", "line"],
            filter=filter_expr,
        )

        search_results = []
        for hit in results[0]:
            search_results.append(SearchResult(
                id=hit["entity"]["id"],
                score=hit["distance"],
                content=hit["entity"]["content"],
                file=hit["entity"]["file"],
                line=hit["entity"]["line"],
            ))

        return search_results

    def delete_by_ids(self, ids: List[str]) -> None:
        """Delete chunks by IDs."""
        for id_ in ids:
            self.client.delete(
                collection_name=self.COLLECTION_NAME,
                pks=[id_],
            )

    def count(self) -> int:
        """Get total chunk count."""
        return self.client.query(
            collection_name=self.COLLECTION_NAME,
            output_fields=["count(*)"],
        )[0]["count(*)"]

    def stats(self) -> Dict[str, Any]:
        """Get collection stats."""
        return {
            "chunks": self.count(),
            "dim": self.dim,
            "collection": self.COLLECTION_NAME,
        }


# Global store instance
_store: Optional[ZillizStore] = None


def get_zilliz_store() -> ZillizStore:
    """Get global Zilliz store instance."""
    global _store

    if _store is not None:
        return _store

    uri = os.environ.get("ZILLIZ_URI")
    token = os.environ.get("ZILLIZ_TOKEN")

    if not uri or not token:
        raise ValueError(
            "ZILLIZ_URI and ZILLIZ_TOKEN must be set. "
            "Get them from https://cloud.zilliz.com"
        )

    _store = ZillizStore(uri=uri, token=token)
    return _store


def is_zilliz_configured() -> bool:
    """Check if Zilliz is configured."""
    uri = os.environ.get("ZILLIZ_URI")
    token = os.environ.get("ZILLIZ_TOKEN")
    return bool(uri and token)
