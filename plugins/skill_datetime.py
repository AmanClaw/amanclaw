#!/usr/bin/env python3
"""Date, time, and timezone utilities."""
from datetime import datetime, timezone, timedelta
from amanclaw_sdk import plugin, SkillInput, SkillResult

TIMEZONES = {
    "MYT": 8, "SGT": 8, "WIB": 7, "WITA": 8, "WIT": 9,
    "JST": 9, "KST": 9, "CST": 8, "IST": 5.5, "AST": 3,
    "UTC": 0, "GMT": 0, "EST": -5, "PST": -8, "CET": 1,
}

@plugin(
    name="datetime_tool",
    description="Get current date/time, convert between timezones, calculate date differences.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["now", "convert", "diff"], "description": "Operation"},
            "timezone": {"type": "string", "description": "Timezone (e.g., MYT, UTC, JST)"},
            "from_tz": {"type": "string", "description": "Source timezone for convert"},
            "to_tz": {"type": "string", "description": "Target timezone for convert"},
            "time": {"type": "string", "description": "Time string (HH:MM) for convert"},
            "date1": {"type": "string", "description": "First date (YYYY-MM-DD) for diff"},
            "date2": {"type": "string", "description": "Second date (YYYY-MM-DD) for diff"}
        },
        "required": ["action"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "now")

    try:
        if action == "now":
            tz_name = args.get("timezone", "MYT").upper()
            offset = TIMEZONES.get(tz_name, 0)
            tz = timezone(timedelta(hours=offset))
            now = datetime.now(tz)
            return SkillResult.ok(
                f"Current time ({tz_name}): {now.strftime('%Y-%m-%d %H:%M:%S %Z')}\n"
                f"Day: {now.strftime('%A')}\n"
                f"Unix timestamp: {int(now.timestamp())}"
            )
        elif action == "convert":
            from_tz = args.get("from_tz", "UTC").upper()
            to_tz = args.get("to_tz", "MYT").upper()
            time_str = args.get("time", "12:00")
            h, m = map(int, time_str.split(":"))
            from_offset = TIMEZONES.get(from_tz, 0)
            to_offset = TIMEZONES.get(to_tz, 0)
            diff = to_offset - from_offset
            result_h = (h + diff) % 24
            return SkillResult.ok(f"{time_str} {from_tz} = {int(result_h):02d}:{m:02d} {to_tz}")
        elif action == "diff":
            d1 = datetime.strptime(args.get("date1", ""), "%Y-%m-%d")
            d2 = datetime.strptime(args.get("date2", ""), "%Y-%m-%d")
            delta = abs((d2 - d1).days)
            return SkillResult.ok(f"Difference: {delta} days ({delta // 7} weeks, {delta // 30} months approx)")
        else:
            return SkillResult.err(f"Unknown action: {action}")
    except Exception as e:
        return SkillResult.err(f"DateTime error: {e}")

if __name__ == "__main__":
    execute.run()
