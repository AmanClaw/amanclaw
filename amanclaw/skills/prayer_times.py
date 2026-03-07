"""
Prayer times skill — fetch prayer times using the Aladhan API (free, no API key).
"""

import logging
import requests
from datetime import datetime
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.prayer_times")

# Common Malaysian cities/states with their coordinates
LOCATIONS = {
    "kuala lumpur": (3.1390, 101.6869),
    "kl": (3.1390, 101.6869),
    "putrajaya": (2.9264, 101.6964),
    "shah alam": (3.0733, 101.5185),
    "selangor": (3.0733, 101.5185),
    "petaling jaya": (3.1073, 101.6067),
    "pj": (3.1073, 101.6067),
    "melaka": (2.1896, 102.2501),
    "malacca": (2.1896, 102.2501),
    "johor bahru": (1.4927, 103.7414),
    "johor": (1.4927, 103.7414),
    "jb": (1.4927, 103.7414),
    "penang": (5.4164, 100.3327),
    "pulau pinang": (5.4164, 100.3327),
    "george town": (5.4164, 100.3327),
    "ipoh": (4.5975, 101.0901),
    "perak": (4.5975, 101.0901),
    "kuantan": (3.8077, 103.3260),
    "pahang": (3.8077, 103.3260),
    "kota bharu": (6.1256, 102.2385),
    "kelantan": (6.1256, 102.2385),
    "kuala terengganu": (5.3117, 103.1324),
    "terengganu": (5.3117, 103.1324),
    "alor setar": (6.1248, 100.3677),
    "kedah": (6.1248, 100.3677),
    "kangar": (6.4414, 100.1986),
    "perlis": (6.4414, 100.1986),
    "seremban": (2.7258, 101.9424),
    "negeri sembilan": (2.7258, 101.9424),
    "kota kinabalu": (5.9804, 116.0735),
    "sabah": (5.9804, 116.0735),
    "kuching": (1.5535, 110.3593),
    "sarawak": (1.5535, 110.3593),
    "cyberjaya": (2.9213, 101.6559),
    "subang jaya": (3.0565, 101.5851),
    "klang": (3.0449, 101.4455),
    "kajang": (2.9927, 101.7909),
    "ampang": (3.1500, 101.7667),
    "cheras": (3.1073, 101.7328),
}

# JAKIM method (Malaysia)
CALCULATION_METHOD = 3  # Muslim World League; use 2 for ISNA
# For Malaysia specifically, method 3 + school 1 (Shafii) is common
SCHOOL = 1  # Shafii


@skill(
    name="prayer_times",
    description=(
        "Get Islamic prayer times (waktu solat) for a location in Malaysia. "
        "Use when the user asks about prayer times, solat times, waktu solat, or azan. "
        "Supports all Malaysian states and major cities."
    ),
    parameters={
        "location": {
            "type": "string",
            "description": "City or state name (e.g. 'Melaka', 'KL', 'Penang', 'Johor Bahru')",
        },
        "date": {
            "type": "string",
            "description": "Date in DD-MM-YYYY format. Leave empty for today.",
            "optional": True,
        },
    },
    timeout=15,
)
def prayer_times(location: str, date: str = None) -> str:
    """Fetch prayer times from Aladhan API."""
    location_lower = location.lower().strip()
    coords = LOCATIONS.get(location_lower)

    if not coords:
        # Try partial match
        for key, val in LOCATIONS.items():
            if location_lower in key or key in location_lower:
                coords = val
                location_lower = key
                break

    if not coords:
        available = ", ".join(sorted(set(
            k.title() for k in LOCATIONS.keys()
            if not any(k == alias for alias in ["kl", "pj", "jb"])
        )))
        return f"Location '{location}' not found. Available locations:\n{available}"

    lat, lng = coords

    if date:
        date_str = date
    else:
        date_str = datetime.now().strftime("%d-%m-%Y")

    try:
        resp = requests.get(
            "https://api.aladhan.com/v1/timings/" + date_str,
            params={
                "latitude": lat,
                "longitude": lng,
                "method": CALCULATION_METHOD,
                "school": SCHOOL,
            },
            timeout=10,
        )
        resp.raise_for_status()
        data = resp.json()
    except requests.RequestException as e:
        logger.error(f"Aladhan API error: {e}")
        return f"Failed to fetch prayer times: {e}"

    if data.get("code") != 200:
        return f"API error: {data.get('status', 'unknown error')}"

    timings = data["data"]["timings"]
    date_info = data["data"]["date"]["readable"]
    hijri = data["data"]["date"]["hijri"]
    hijri_date = f"{hijri['day']} {hijri['month']['en']} {hijri['year']} H"

    return (
        f"Prayer Times for {location.title()} — {date_info} ({hijri_date})\n\n"
        f"Imsak:   {timings['Imsak']}\n"
        f"Subuh:   {timings['Fajr']}\n"
        f"Syuruk:  {timings['Sunrise']}\n"
        f"Zohor:   {timings['Dhuhr']}\n"
        f"Asar:    {timings['Asr']}\n"
        f"Maghrib: {timings['Maghrib']}\n"
        f"Isyak:   {timings['Isha']}"
    )
