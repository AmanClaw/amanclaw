#!/usr/bin/env python3
"""AmanClaw Plugin: Weekly JAKIM Khutbah.

Get latest weekly Friday khutbah (sermon) from JAKIM Malaysia.
Search khutbah archive.
"""

import json
import urllib.request
from amanclaw_sdk import plugin, SkillInput, SkillResult


@plugin(
    name="khutbah",
    description="Get latest weekly Friday khutbah (sermon) from JAKIM Malaysia. Search khutbah archive. Dapatkan teks khutbah Jumaat terkini dari JAKIM.",
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["latest", "search"],
                "description": "latest = this week's khutbah, search = search archive by keyword",
            },
            "query": {
                "type": "string",
                "description": "Search keyword for khutbah archive",
            },
        },
        "required": [],
    },
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "latest")

    if action == "latest":
        url = "https://www.islam.gov.my/api/khutbah/latest"
        try:
            req = urllib.request.Request(url)
            req.add_header("User-Agent", "AmanClaw/1.0")
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read().decode())
            title = data.get("title", "N/A")
            date = data.get("date", "N/A")
            content = data.get("content", data.get("summary", "Tidak dapat dimuatkan."))
            # Truncate if too long
            if len(content) > 1500:
                content = content[:1500] + "...\n\n[Baca penuh di portal JAKIM / Read full text at JAKIM portal]"
            return SkillResult.ok(
                f"Khutbah Jumaat Minggu Ini / This Week's Friday Sermon:\n\n"
                f"Tajuk / Title: {title}\n"
                f"Tarikh / Date: {date}\n\n"
                f"{content}"
            )
        except Exception:
            return SkillResult.ok(
                "Maaf, tidak dapat memuat khutbah terkini dari JAKIM. / "
                "Sorry, unable to load latest khutbah from JAKIM.\n"
                "Sila layari / Please visit: https://www.islam.gov.my/e-jakim/teks-khutbah-jumaat"
            )

    if action == "search":
        query = args.get("query", "")
        if not query:
            return SkillResult.err(
                "Sila berikan kata kunci carian. / Please provide a search keyword."
            )
        return SkillResult.ok(
            f"Carian khutbah untuk / Khutbah search for '{query}':\n"
            f"Sila layari / Please visit: https://www.islam.gov.my/e-jakim/teks-khutbah-jumaat\n"
            f"dan cari menggunakan kata kunci tersebut. / and search using the keyword."
        )

    return SkillResult.err(
        f"Action tidak dikenali / Unknown action: {action}"
    )


if __name__ == "__main__":
    execute.run()
