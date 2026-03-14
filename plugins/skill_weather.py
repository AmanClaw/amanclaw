#!/usr/bin/env python3
"""Weather using Open-Meteo API (free, no API key)."""
import urllib.request
import urllib.parse
import json
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="weather",
    description="Get current weather and forecast for any city. No API key needed.",
    parameters={
        "type": "object",
        "properties": {
            "city": {"type": "string", "description": "City name (e.g., 'Kuala Lumpur', 'London')"},
            "days": {"type": "integer", "description": "Forecast days (1-7, default 1)"}
        },
        "required": ["city"]
    },
    timeout_ms=15000
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    city = args.get("city", "")
    days = min(args.get("days", 1), 7)

    if not city:
        return SkillResult.err("Please provide a city name.")

    try:
        # Geocode city
        geo_url = f"https://geocoding-api.open-meteo.com/v1/search?name={urllib.parse.quote(city)}&count=1"
        req = urllib.request.Request(geo_url, headers={"User-Agent": "AmanClaw/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            geo = json.loads(resp.read().decode())

        if not geo.get("results"):
            return SkillResult.err(f"City not found: {city}")

        loc = geo["results"][0]
        lat, lon = loc["latitude"], loc["longitude"]
        name = loc.get("name", city)
        country = loc.get("country", "")

        # Get weather
        wx_url = (
            f"https://api.open-meteo.com/v1/forecast?"
            f"latitude={lat}&longitude={lon}"
            f"&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code"
            f"&daily=temperature_2m_max,temperature_2m_min,weather_code"
            f"&forecast_days={days}&timezone=auto"
        )
        req = urllib.request.Request(wx_url, headers={"User-Agent": "AmanClaw/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            wx = json.loads(resp.read().decode())

        current = wx.get("current", {})
        temp = current.get("temperature_2m", "?")
        humidity = current.get("relative_humidity_2m", "?")
        wind = current.get("wind_speed_10m", "?")

        output = f"Weather for {name}, {country}:\n\n"
        output += f"Now: {temp}\u00b0C, Humidity {humidity}%, Wind {wind} km/h\n"

        daily = wx.get("daily", {})
        dates = daily.get("time", [])
        maxes = daily.get("temperature_2m_max", [])
        mins = daily.get("temperature_2m_min", [])

        if dates:
            output += "\nForecast:\n"
            for i, date in enumerate(dates):
                output += f"  {date}: {mins[i]}\u00b0C \u2013 {maxes[i]}\u00b0C\n"

        return SkillResult.ok(output)
    except Exception as e:
        return SkillResult.err(f"Weather error: {e}")

if __name__ == "__main__":
    execute.run()
