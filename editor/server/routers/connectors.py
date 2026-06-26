"""Router: /api/connector/generate."""

from fastapi import APIRouter, Request

from editor.server.models.requests import ConnectorRequest
from editor.server.connectors import (
    ConnectorConfig, EndpointDef, FieldDef,
    generate_rest_store, generate_graphql_store, generate_firebase_store,
    infer_type_from_json,
)

router = APIRouter()


@router.post("/api/connector/generate")
async def api_connector_generate(req: ConnectorRequest, request: Request):
    """Generate data connector scaffold code. Supports REST / GraphQL / Firebase."""
    try:
        endpoints: list[EndpointDef] = []
        for ep in req.endpoints:
            response_fields: list[FieldDef] = []
            if ep.get("responseJson"):
                response_fields = infer_type_from_json(ep["responseJson"])
            endpoints.append(EndpointDef(
                method=ep.get("method", "GET"),
                path=ep.get("path", ""),
                action=ep.get("action", "fetch"),
                requestType=ep.get("requestType", ""),
                responseFields=response_fields,
            ))

        config = ConnectorConfig(
            type=req.type,
            name=req.name,
            baseUrl=req.baseUrl,
            endpoints=endpoints,
            query=req.query,
            collection=req.collection,
        )

        if req.type == "rest":
            code = generate_rest_store(config)
        elif req.type == "graphql":
            code = generate_graphql_store(config)
        elif req.type == "firebase":
            code = generate_firebase_store(config)
        else:
            return {"success": False, "code": "", "error": f"Unsupported connector type: {req.type}"}

        return {"success": True, "code": code}
    except Exception as e:
        return {"success": False, "code": "", "error": str(e)}
