"""Pydantic request/response models for the Editor API.

Extracted from editor/server/main.py during the router-split refactor.
"""

from pydantic import BaseModel


class CompileRequest(BaseModel):
    source: str
    target: str = "react"
    previousFiles: dict[str, str] | None = None
    debug: bool = False  # Enable runtime debug injection (useTrackedSignal etc.)


class FsReadRequest(BaseModel):
    filePath: str


class FsListRequest(BaseModel):
    dirPath: str = "."


class FsWriteRequest(BaseModel):
    filePath: str
    content: str


class ListDirectoryRequest(BaseModel):
    path: str = "."


class SetProjectDirRequest(BaseModel):
    dir: str


class CreateProjectRequest(BaseModel):
    dir: str
    name: str


class CodeOwnershipRequest(BaseModel):
    source: str
    action: str = "parse"  # "parse" | "parse-ast" | "apply-edit"
    editStartLine: int = 0
    editEndLine: int = 0
    newContent: str = ""


class ComponentDefinitionsPayload(BaseModel):
    definitions: dict = {}


class ComponentDeletePayload(BaseModel):
    names: list[str]


class AnnotatedSaveRequest(BaseModel):
    """POST /api/annotated-save request body"""
    filePath: str
    annotations: dict  # Serialized AnnotationFile
    source: str         # The .omni source content


class AnnotatedLoadRequest(BaseModel):
    """POST /api/annotated-load request body"""
    filePath: str


class EditorTomlSaveRequest(BaseModel):
    """POST /api/editor-toml/save request body"""
    state: dict  # Serialized EditorTomlState


class ConnectorRequest(BaseModel):
    """POST /api/connector/generate request body"""
    type: str  # "rest" | "graphql" | "firebase"
    name: str
    baseUrl: str = ""
    endpoints: list[dict] = []
    query: str = ""
    collection: str = ""
