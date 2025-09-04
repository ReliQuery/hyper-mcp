# mcp_call.py
import argparse, asyncio, json, sys
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

async def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--server-cmd", required=True)
    ap.add_argument("--server-arg", action="append", default=[])
    ap.add_argument("--tool", required=True)
    ap.add_argument("--tool-args", default="{}")
    args = ap.parse_args()

    params = StdioServerParameters(command=args.server_cmd, args=args.server_arg)
    try:
        async with stdio_client(params) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()
                result = await session.call_tool(args.tool, json.loads(args.tool_args))
                # result is a Pydantic model; dump stable JSON for CI
                print(result.model_dump_json())
                return 0
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
