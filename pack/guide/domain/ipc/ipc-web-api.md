# Web API

Web APIs are mostly based on JSON-RPC, typically exposed on `/wapi/rpc` or `/api/rpc`.

- `/wapi/rpc` is for the web UI, and typically relies on an HTTP-only token authentication scheme.
- `/api/rpc` is for the public API, and typically relies on an API token or API key authentication scheme.

The reason to split the two is the following:

- They have different lifecycles. `/wapi/` lives with the UI, and does not have to be as backward compatible as the public API.
- `/api/` needs to be stable, and usually requires a different authentication flow.
- It allows mapping them to different deployments so we can limit what the API or WAPI can do.

There are some exceptions for login and logoff flows, which can be posted to:

- `/wapi/login`
- `/wapi/logoff`

When downloading content such as CSV files, ZIP files, and similar assets, those will usually follow a more REST-style API, probably something like `download/_uuid_` or a similar endpoint shape.
