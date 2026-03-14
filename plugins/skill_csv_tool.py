#!/usr/bin/env python3
"""CSV parsing, formatting, and analysis."""
import csv
import io
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="csv_tool",
    description="Parse CSV data, convert to table, get stats, extract columns.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["parse", "stats", "column", "to_json"], "description": "Operation"},
            "data": {"type": "string", "description": "CSV data as string"},
            "column": {"type": "string", "description": "Column name (for column/stats)"}
        },
        "required": ["action", "data"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "parse")
    data = args.get("data", "")

    try:
        reader = csv.DictReader(io.StringIO(data))
        rows = list(reader)

        if not rows:
            return SkillResult.err("No data found in CSV.")

        if action == "parse":
            headers = list(rows[0].keys())
            output = f"Columns: {', '.join(headers)}\nRows: {len(rows)}\n\n"
            for i, row in enumerate(rows[:10]):
                output += f"Row {i+1}: {dict(row)}\n"
            if len(rows) > 10:
                output += f"\n... and {len(rows) - 10} more rows"
            return SkillResult.ok(output)

        elif action == "stats":
            col = args.get("column", "")
            if col and col in rows[0]:
                values = [r[col] for r in rows if r.get(col)]
                try:
                    nums = [float(v) for v in values]
                    avg = sum(nums) / len(nums)
                    return SkillResult.ok(
                        f"Column '{col}': {len(nums)} values\n"
                        f"Min: {min(nums)}, Max: {max(nums)}, Avg: {avg:.2f}, Sum: {sum(nums)}"
                    )
                except ValueError:
                    unique = len(set(values))
                    return SkillResult.ok(f"Column '{col}': {len(values)} values, {unique} unique")
            return SkillResult.ok(f"Available columns: {', '.join(rows[0].keys())}")

        elif action == "column":
            col = args.get("column", "")
            if col not in rows[0]:
                return SkillResult.err(f"Column '{col}' not found. Available: {', '.join(rows[0].keys())}")
            values = [r[col] for r in rows]
            return SkillResult.ok("\n".join(values))

        elif action == "to_json":
            import json
            return SkillResult.ok(json.dumps(rows, indent=2))

        return SkillResult.err(f"Unknown action: {action}")
    except Exception as e:
        return SkillResult.err(f"CSV error: {e}")

if __name__ == "__main__":
    execute.run()
