#!/usr/bin/env python3
"""AmanClaw Plugin: JAKIM Services & Fatwa Search.

Access JAKIM Malaysia services: fatwa search, Islamic events calendar,
e-JAKIM services directory.
"""

import json
import urllib.request
import urllib.parse
from amanclaw_sdk import plugin, SkillInput, SkillResult


@plugin(
    name="jakim",
    description="Access JAKIM Malaysia services: fatwa search, Islamic events calendar, e-JAKIM services directory. Akses perkhidmatan JAKIM: carian fatwa, kalendar Islam, direktori e-JAKIM.",
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["fatwa", "events", "services"],
                "description": "fatwa = search fatwa database, events = Islamic events calendar, services = JAKIM services directory",
            },
            "query": {
                "type": "string",
                "description": "Search query for fatwa",
            },
        },
        "required": [],
    },
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "services")

    if action == "services":
        return SkillResult.ok(
            "Perkhidmatan e-JAKIM / e-JAKIM Services:\n\n"
            "1. e-Solat - Waktu solat seluruh Malaysia / Prayer times for all Malaysia\n"
            "   https://www.e-solat.gov.my\n\n"
            "2. Portal Halal - Semakan status halal / Halal status verification\n"
            "   https://www.halal.gov.my\n\n"
            "3. e-Fatwa - Pangkalan data fatwa Malaysia / Malaysia fatwa database\n"
            "   https://e-muamalat.islam.gov.my\n\n"
            "4. Teks Khutbah - Khutbah Jumaat mingguan / Weekly Friday sermons\n"
            "   https://www.islam.gov.my/e-jakim/teks-khutbah-jumaat\n\n"
            "5. e-Quran - Al-Quran digital JAKIM / JAKIM digital Quran\n"
            "   https://quran.jakim.gov.my\n\n"
            "6. SPMJ - Sistem Pengurusan Masjid / Mosque Management System\n"
            "   https://spmj.jawhar.gov.my"
        )

    if action == "fatwa":
        query = args.get("query", "")
        if not query:
            return SkillResult.err(
                "Sila berikan kata kunci untuk carian fatwa. / "
                "Please provide a keyword for fatwa search."
            )
        try:
            encoded_query = urllib.parse.quote(query)
            url = f"https://e-muamalat.islam.gov.my/api/fatwa/search?q={encoded_query}"
            req = urllib.request.Request(url)
            req.add_header("User-Agent", "AmanClaw/1.0")
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read().decode())
            results = data.get("results", data.get("data", []))
            if not results:
                return SkillResult.ok(
                    f"Tiada fatwa ditemui untuk '{query}'. / No fatwa found for '{query}'.\n"
                    f"Cuba cari di / Try searching at: https://e-muamalat.islam.gov.my"
                )
            lines = []
            for r in results[:3]:
                title = r.get("title", "N/A")
                status = r.get("status", "N/A")
                date = r.get("date", "N/A")
                lines.append(
                    f"- {title}\n  Status: {status} | Tarikh / Date: {date}"
                )
            return SkillResult.ok(
                f"Fatwa berkaitan / Fatwa related to '{query}':\n\n"
                + "\n\n".join(lines)
            )
        except Exception:
            return SkillResult.ok(
                f"Tidak dapat mencari fatwa secara automatik. / "
                f"Unable to search fatwa automatically.\n"
                f"Sila layari / Please visit: https://e-muamalat.islam.gov.my "
                f"dan cari / and search for '{query}'."
            )

    if action == "events":
        return SkillResult.ok(
            "Peristiwa Islam Utama / Major Islamic Events:\n\n"
            "- Awal Muharram (Tahun Baru Islam / Islamic New Year)\n"
            "- Mawlidur Rasul (Hari Keputeraan Nabi / Prophet's Birthday)\n"
            "- Israk & Mikraj\n"
            "- Nisfu Sya'ban\n"
            "- Ramadan\n"
            "- Nuzul Al-Quran\n"
            "- Hari Raya Aidilfitri\n"
            "- Hari Raya Haji (Aidiladha)\n\n"
            "Tarikh tepat bergantung pada rukyah. / "
            "Exact dates depend on moon sighting.\n"
            "Semak di / Check at: https://www.islam.gov.my"
        )

    return SkillResult.err(
        f"Action tidak dikenali / Unknown action: {action}"
    )


if __name__ == "__main__":
    execute.run()
