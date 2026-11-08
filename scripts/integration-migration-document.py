#!/usr/bin/env python3
"""Exercise guided migration and document ingestion against a live test API."""

import json
import pathlib
import sys
import urllib.error
import urllib.request

base, root = sys.argv[1:3]


def request(method, path, body=None, content_type="application/json"):
    data = body
    if body is not None and content_type == "application/json":
        data = json.dumps(body).encode()
    req = urllib.request.Request(base + path, data=data, method=method)
    if data is not None:
        req.add_header("content-type", content_type)
    try:
        with urllib.request.urlopen(req) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        raise SystemExit(f"{method} {path} failed: {error.read().decode()}")


rows = json.loads(
    (pathlib.Path(root) / "samples/migration-portfolio.json").read_text()
)
status, body = request(
    "POST",
    "/api/v1/migrations/preview",
    {"source": "generic_json", "mapping": {}, "content": json.dumps(rows)},
)
assert status == 200, body
preview = json.loads(body)
assert preview["accounts"] == 3 and preview["balances"] == 1, preview

status, body = request(
    "POST",
    "/api/v1/migrations/apply",
    {
        "source": "generic_json",
        "mapping": {},
        "content": json.dumps(rows),
        "mode": "atomic",
        "backup_confirmed": True,
    },
)
assert status == 200, body
report = json.loads(body)
assert report["accounts_created"] == 3, report

csv = (
    "record_type,account_name,account_type,as_of,amount,source\n"
    "account,Document Import Checking,checking,,,\n"
    "balance,Document Import Checking,,2026-01-01,321.00,import\n"
).encode()
status, body = request(
    "POST",
    "/api/v1/documents?filename=fidelity-document.csv&tags=integration",
    csv,
    "text/csv",
)
assert status == 201, body
document = json.loads(body)
assert document["institution"] == "Fidelity", document

status, body = request("POST", f"/api/v1/documents/{document['id']}/preview")
assert status == 200, body
parsed = json.loads(body)
assert parsed["format"] == "csv", parsed
assert parsed["table"]["rows"][0][0] == "account", parsed

status, body = request(
    "POST",
    f"/api/v1/documents/{document['id']}/import",
    {"mapping": {}, "mode": "atomic", "backup_confirmed": True},
)
assert status == 200, body
imported = json.loads(body)
assert imported["balances_saved"] == 1, imported

print("Migration and document API integration passed.")
