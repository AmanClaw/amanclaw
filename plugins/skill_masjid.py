#!/usr/bin/env python3
"""AmanClaw Plugin: Mosque Finder via Google Places API.

Find nearest masjid (mosque) or surau from a location in Malaysia.
"""

import json
import os
import urllib.request
import urllib.parse
from amanclaw_sdk import plugin, SkillInput, SkillResult


@plugin(
    name="masjid",
    description="Find nearest masjid (mosque) or surau from a location in Malaysia using Google Places API. Cari masjid/surau berhampiran.",
    parameters={
        "type": "object",
        "properties": {
            "latitude": {
                "type": "number",
                "description": "Latitude of current location",
            },
            "longitude": {
                "type": "number",
                "description": "Longitude of current location",
            },
            "location": {
                "type": "string",
                "description": "Location name e.g. 'Shah Alam', 'KLCC'. Used if lat/lon not provided.",
            },
            "radius": {
                "type": "integer",
                "description": "Search radius in meters (default: 2000)",
            },
        },
        "required": [],
    },
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    api_key = os.environ.get("GOOGLE_PLACES_API_KEY", "")

    if not api_key:
        return SkillResult.err(
            "GOOGLE_PLACES_API_KEY tidak dikonfigurasi. / "
            "GOOGLE_PLACES_API_KEY not configured."
        )

    lat = args.get("latitude")
    lon = args.get("longitude")
    location = args.get("location", "")
    radius = args.get("radius", 2000)

    if not isinstance(radius, int) or radius <= 0:
        radius = 2000

    # Geocode location name if no coordinates
    if (lat is None or lon is None) and location:
        geo_url = (
            f"https://maps.googleapis.com/maps/api/geocode/json"
            f"?address={urllib.parse.quote(location + ', Malaysia')}&key={api_key}"
        )
        req = urllib.request.Request(geo_url)
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode())
                if data.get("results"):
                    loc = data["results"][0]["geometry"]["location"]
                    lat, lon = loc["lat"], loc["lng"]
        except Exception as e:
            return SkillResult.err(
                f"Gagal geocode lokasi / Failed to geocode location: {e}"
            )

    if lat is None or lon is None:
        return SkillResult.err(
            "Sila berikan lokasi (latitude/longitude atau nama tempat). / "
            "Please provide a location (lat/lon or place name)."
        )

    # Search for mosques nearby
    places_url = (
        f"https://maps.googleapis.com/maps/api/place/nearbysearch/json"
        f"?location={lat},{lon}&radius={radius}&type=mosque&key={api_key}&language=ms"
    )
    req = urllib.request.Request(places_url)
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode())
    except Exception as e:
        return SkillResult.err(
            f"Gagal mencari masjid / Failed to search for mosques: {e}"
        )

    results = data.get("results", [])
    if not results:
        return SkillResult.ok(
            f"Tiada masjid/surau ditemui dalam radius {radius}m dari lokasi anda. "
            f"Cuba tingkatkan radius. / "
            f"No mosque/surau found within {radius}m. Try increasing radius."
        )

    lines = []
    for r in results[:5]:
        name = r.get("name", "N/A")
        addr = r.get("vicinity", "N/A")
        rating = r.get("rating", "N/A")
        open_now = r.get("opening_hours", {}).get("open_now")
        if open_now is True:
            status = "Buka / Open"
        elif open_now is False:
            status = "Tutup / Closed"
        else:
            status = "Tidak pasti / Unknown"
        rlat = r["geometry"]["location"]["lat"]
        rlon = r["geometry"]["location"]["lng"]
        maps_link = f"https://maps.google.com/?q={rlat},{rlon}"
        lines.append(
            f"- {name}\n"
            f"  Alamat / Address: {addr}\n"
            f"  Rating: {rating} | Status: {status}\n"
            f"  Maps: {maps_link}"
        )

    header = (
        f"Masjid/Surau berhampiran / Nearby mosques "
        f"({len(results)} ditemui / found):"
    )
    return SkillResult.ok(header + "\n\n" + "\n\n".join(lines))


if __name__ == "__main__":
    execute.run()
