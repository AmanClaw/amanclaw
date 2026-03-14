#!/usr/bin/env python3
"""Unit conversion: length, weight, temperature, currency concepts."""
from amanclaw_sdk import plugin, SkillInput, SkillResult

CONVERSIONS = {
    "km_to_mi": 0.621371, "mi_to_km": 1.60934,
    "kg_to_lb": 2.20462, "lb_to_kg": 0.453592,
    "m_to_ft": 3.28084, "ft_to_m": 0.3048,
    "cm_to_in": 0.393701, "in_to_cm": 2.54,
    "l_to_gal": 0.264172, "gal_to_l": 3.78541,
    "g_to_oz": 0.035274, "oz_to_g": 28.3495,
}

@plugin(
    name="unit_convert",
    description="Convert between units: length (km/mi/m/ft/cm/in), weight (kg/lb/g/oz), volume (l/gal), temperature (C/F/K).",
    parameters={
        "type": "object",
        "properties": {
            "value": {"type": "number", "description": "Value to convert"},
            "from_unit": {"type": "string", "description": "Source unit"},
            "to_unit": {"type": "string", "description": "Target unit"}
        },
        "required": ["value", "from_unit", "to_unit"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    value = args.get("value", 0)
    from_u = args.get("from_unit", "").lower()
    to_u = args.get("to_unit", "").lower()

    try:
        # Temperature special cases
        if from_u == "c" and to_u == "f":
            result = value * 9/5 + 32
        elif from_u == "f" and to_u == "c":
            result = (value - 32) * 5/9
        elif from_u == "c" and to_u == "k":
            result = value + 273.15
        elif from_u == "k" and to_u == "c":
            result = value - 273.15
        else:
            key = f"{from_u}_to_{to_u}"
            if key not in CONVERSIONS:
                return SkillResult.err(f"Unknown conversion: {from_u} → {to_u}")
            result = value * CONVERSIONS[key]

        return SkillResult.ok(f"{value} {from_u} = {result:.4f} {to_u}")
    except Exception as e:
        return SkillResult.err(f"Conversion error: {e}")

if __name__ == "__main__":
    execute.run()
