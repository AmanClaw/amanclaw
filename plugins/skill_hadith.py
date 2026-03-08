#!/usr/bin/env python3
"""AmanClaw Plugin: Hadith Search via sunnah.com API.

Search and lookup hadith from major collections (Bukhari, Muslim, Abu Dawud,
Tirmidhi, Nasai, Ibn Majah).
"""

import json
import os
import urllib.request
import urllib.parse
from amanclaw_sdk import plugin, SkillInput, SkillResult

SUNNAH_API = "https://api.sunnah.com/v1"

COLLECTIONS = {
    "bukhari": {"name": "Sahih al-Bukhari", "id": "bukhari"},
    "muslim": {"name": "Sahih Muslim", "id": "muslim"},
    "abudawud": {"name": "Sunan Abu Dawud", "id": "abudawud"},
    "tirmidhi": {"name": "Jami` at-Tirmidhi", "id": "tirmidhi"},
    "nasai": {"name": "Sunan an-Nasa'i", "id": "nasai"},
    "ibnmajah": {"name": "Sunan Ibn Majah", "id": "ibnmajah"},
}


def api_get(path, api_key=""):
    """Make a GET request to the sunnah.com API."""
    url = f"{SUNNAH_API}{path}"
    req = urllib.request.Request(url)
    if api_key:
        req.add_header("x-api-key", api_key)
    req.add_header("Accept", "application/json")
    with urllib.request.urlopen(req, timeout=15) as resp:
        return json.loads(resp.read().decode())


@plugin(
    name="hadith",
    description="Search and lookup hadith from major collections (Bukhari, Muslim, Abu Dawud, Tirmidhi, Nasai, Ibn Majah). Cari dan rujuk hadis dari koleksi utama.",
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["search", "lookup", "random", "collections"],
                "description": "search = search by keyword, lookup = specific hadith by number, random = random hadith, collections = list available collections",
            },
            "query": {"type": "string", "description": "Search keyword"},
            "collection": {
                "type": "string",
                "description": "Collection name: bukhari, muslim, abudawud, tirmidhi, nasai, ibnmajah",
            },
            "hadith_number": {
                "type": "string",
                "description": "Hadith number for lookup",
            },
        },
        "required": [],
    },
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "search")
    api_key = os.environ.get("SUNNAH_API_KEY", "")

    if action == "collections":
        lines = [f"- {k}: {v['name']}" for k, v in COLLECTIONS.items()]
        return SkillResult.ok(
            "Koleksi Hadis / Hadith Collections:\n" + "\n".join(lines)
        )

    if action == "lookup":
        collection = args.get("collection", "bukhari")
        number = args.get("hadith_number", "1")
        if collection not in COLLECTIONS:
            return SkillResult.err(
                f"Koleksi '{collection}' tidak dikenali. "
                f"Koleksi tersedia: {', '.join(COLLECTIONS.keys())}"
            )
        try:
            data = api_get(f"/collections/{collection}/hadiths/{number}", api_key)
            hadith = data.get("hadith", [data])[0] if isinstance(data, dict) else data
            text_ar = hadith.get("arabicText", hadith.get("body", ""))
            text_en = hadith.get("englishText", hadith.get("text", ""))
            col_name = COLLECTIONS.get(collection, {}).get("name", collection)
            return SkillResult.ok(
                f"{col_name} #{number}\n\n"
                f"Arab:\n{text_ar}\n\n"
                f"English:\n{text_en}"
            )
        except Exception as e:
            return SkillResult.err(
                f"Gagal mendapatkan hadis / Failed to lookup hadith: {e}"
            )

    if action == "search":
        query = args.get("query", "")
        if not query:
            return SkillResult.err(
                "Sila berikan kata kunci carian / Please provide a search query."
            )
        try:
            encoded_query = urllib.parse.quote(query)
            data = api_get(f"/hadiths?q={encoded_query}&limit=3", api_key)
            hadiths = data.get("data", [])
            if not hadiths:
                return SkillResult.ok(
                    f"Tiada hadis ditemui untuk '{query}'. / No hadith found for '{query}'."
                )
            results = []
            for h in hadiths[:3]:
                col = h.get("collection", "?")
                num = h.get("hadithNumber", "?")
                text = h.get("englishText", h.get("body", ""))[:200]
                results.append(f"[{col} #{num}]\n{text}...")
            return SkillResult.ok(
                f"Hasil carian / Search results for '{query}':\n\n"
                + "\n\n".join(results)
            )
        except Exception as e:
            return SkillResult.err(f"Carian gagal / Search failed: {e}")

    if action == "random":
        try:
            data = api_get("/hadiths/random", api_key)
            col = data.get("collection", "?")
            num = data.get("hadithNumber", "?")
            text_en = data.get("englishText", data.get("body", ""))
            text_ar = data.get("arabicText", "")
            return SkillResult.ok(
                f"Hadis Rawak / Random Hadith [{col} #{num}]:\n\n"
                f"Arab:\n{text_ar}\n\n"
                f"English:\n{text_en}"
            )
        except Exception as e:
            return SkillResult.err(
                f"Gagal mendapatkan hadis rawak / Failed to get random hadith: {e}"
            )

    return SkillResult.err(
        f"Action tidak dikenali / Unknown action: {action}"
    )


if __name__ == "__main__":
    execute.run()
