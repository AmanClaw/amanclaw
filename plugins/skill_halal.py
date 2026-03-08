#!/usr/bin/env python3
"""AmanClaw Plugin: JAKIM Halal Verification.

Check halal status of products, restaurants, and premises using
JAKIM Malaysia halal database. Verify halal certificates.
"""

import json
import urllib.request
import urllib.parse
from amanclaw_sdk import plugin, SkillInput, SkillResult

JAKIM_HALAL_URL = "https://www.halal.gov.my/v4/api"


def search_halal(query):
    """Search JAKIM halal directory."""
    encoded = urllib.parse.quote(query)
    url = f"{JAKIM_HALAL_URL}/search?keyword={encoded}&type=all"
    req = urllib.request.Request(url)
    req.add_header("Accept", "application/json")
    req.add_header("User-Agent", "AmanClaw/1.0")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode())
    except Exception:
        return None


def verify_cert(cert_number):
    """Verify halal certificate by number."""
    url = f"{JAKIM_HALAL_URL}/verify?cert={urllib.parse.quote(cert_number)}"
    req = urllib.request.Request(url)
    req.add_header("Accept", "application/json")
    req.add_header("User-Agent", "AmanClaw/1.0")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode())
    except Exception:
        return None


@plugin(
    name="halal",
    description="Check halal status of products, restaurants, and premises using JAKIM Malaysia halal database. Semak status halal produk dan restoran melalui pangkalan data JAKIM.",
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["search", "verify"],
                "description": "search = search by product/restaurant name, verify = verify by certificate number",
            },
            "query": {
                "type": "string",
                "description": "Product, restaurant, or company name to search",
            },
            "cert_number": {
                "type": "string",
                "description": "JAKIM halal certificate number to verify",
            },
        },
        "required": [],
    },
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "search")

    if action == "verify":
        cert = args.get("cert_number", "")
        if not cert:
            return SkillResult.err(
                "Sila berikan nombor sijil halal / Please provide certificate number."
            )
        data = verify_cert(cert)
        if data is None:
            return SkillResult.err(
                "Gagal menghubungi pangkalan data JAKIM. Sila cuba lagi. / "
                "Failed to connect to JAKIM database. Please try again."
            )
        if data.get("valid"):
            return SkillResult.ok(
                f"Sijil Halal JAKIM / JAKIM Halal Certificate: {cert}\n"
                f"Status: SAH / VALID\n"
                f"Syarikat / Company: {data.get('company', 'N/A')}\n"
                f"Tamat / Expiry: {data.get('expiry', 'N/A')}"
            )
        else:
            return SkillResult.ok(
                f"Sijil {cert}: TIDAK SAH / NOT VALID atau tidak ditemui / or not found."
            )

    # Default: search
    query = args.get("query", "")
    if not query:
        return SkillResult.err(
            "Sila berikan nama produk/restoran untuk dicari. / "
            "Please provide a product/restaurant name to search."
        )
    data = search_halal(query)
    if data is None:
        return SkillResult.err(
            "Gagal menghubungi pangkalan data JAKIM. Sila cuba lagi. / "
            "Failed to connect to JAKIM database. Please try again."
        )
    results = data.get("results", data.get("data", []))
    if not results:
        return SkillResult.ok(
            f"Tiada keputusan halal ditemui untuk '{query}'. "
            f"Cuba nama lain atau semak di halal.gov.my. / "
            f"No halal results found for '{query}'. Try another name or check halal.gov.my."
        )
    lines = []
    for r in results[:5]:
        name = r.get("name", r.get("company", "N/A"))
        status = r.get("status", "N/A")
        cert = r.get("cert_number", r.get("certificate", "N/A"))
        expiry = r.get("expiry", r.get("valid_until", "N/A"))
        lines.append(
            f"- {name}\n  Sijil: {cert} | Status: {status} | Tamat: {expiry}"
        )
    return SkillResult.ok(
        f"Keputusan carian halal untuk '{query}' / Halal search results:\n\n"
        + "\n".join(lines)
    )


if __name__ == "__main__":
    execute.run()
