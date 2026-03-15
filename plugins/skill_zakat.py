#!/usr/bin/env python3
"""AmanClaw Plugin: Zakat Calculator.

Calculate zakat (Islamic tax): fitrah, pendapatan (income),
simpanan (savings), emas (gold), perniagaan (business),
pertanian (agriculture), and ternakan (livestock).
"""

from amanclaw_sdk import plugin, SkillInput, SkillResult

# 2024/2025 rates - update yearly from JAKIM/state zakat authorities
ZAKAT_FITRAH_RATES = {
    "default": 7.00,
    "WLY": 7.00, "SGR": 7.00, "JHR": 7.00, "PHG": 7.00,
    "PRK": 7.00, "KDH": 7.00, "KTN": 7.00, "TRG": 7.00,
    "PNG": 7.00, "PLS": 7.00, "MLK": 7.00, "NGS": 7.00,
    "SBH": 7.00, "SWK": 7.00,
}

NISAB_GOLD_GRAMS = 85.0  # 85 grams of gold
DEFAULT_GOLD_PRICE_PER_GRAM = 400.0  # RM per gram — override via gold_price_per_gram param

# Livestock nisab thresholds
LIVESTOCK_NISAB = {
    "goat": {"nisab": 40, "zakat": "1 ekor kambing (1 goat) per 40 ekor"},
    "cattle": {"nisab": 30, "zakat": "1 ekor lembu tabi' (1 young cattle) per 30 ekor"},
    "camel": {"nisab": 5, "zakat": "1 ekor kambing (1 goat) per 5 ekor unta"},
}


@plugin(
    name="zakat",
    description=(
        "Calculate zakat (Islamic tax). Supports fitrah, pendapatan (income), "
        "simpanan (savings), emas (gold), perniagaan (business), pertanian "
        "(agriculture), ternakan (livestock), and info_nisab. "
        "Kira zakat fitrah, pendapatan, simpanan, emas, perniagaan, pertanian, dan ternakan."
    ),
    parameters={
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "enum": [
                    "fitrah", "pendapatan", "simpanan", "emas",
                    "perniagaan", "pertanian", "ternakan",
                    "info", "info_nisab",
                ],
                "description": "Type of zakat to calculate",
            },
            "state": {
                "type": "string",
                "description": "State code for fitrah rate (e.g., WLY, SGR, JHR)",
            },
            "dependents": {
                "type": "integer",
                "description": "Number of dependents for fitrah",
            },
            "annual_income": {
                "type": "number",
                "description": "Annual gross income (RM) for zakat pendapatan",
            },
            "annual_expenses": {
                "type": "number",
                "description": "Annual allowable expenses/deductions (RM)",
            },
            "savings_balance": {
                "type": "number",
                "description": "Lowest savings balance in the year (RM)",
            },
            "gold_grams": {
                "type": "number",
                "description": "Gold weight in grams for zakat emas",
            },
            "gold_price_per_gram": {
                "type": "number",
                "description": "Current gold price per gram (RM) — default 400",
            },
            "net_business_assets": {
                "type": "number",
                "description": "Net business assets (RM) = current assets - current liabilities",
            },
            "harvest_kg": {
                "type": "number",
                "description": "Harvest weight in kg for zakat pertanian",
            },
            "irrigation": {
                "type": "string",
                "enum": ["rain", "irrigated", "mixed"],
                "description": "Irrigation method: rain (10%), irrigated (5%), mixed (7.5%)",
            },
            "livestock_type": {
                "type": "string",
                "enum": ["goat", "cattle", "camel"],
                "description": "Type of livestock for zakat ternakan",
            },
            "livestock_count": {
                "type": "integer",
                "description": "Number of livestock owned",
            },
        },
        "required": [],
    },
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    zakat_type = args.get("type", "info")

    if zakat_type == "info":
        return SkillResult.ok(
            "Jenis-jenis Zakat / Types of Zakat:\n\n"
            "1. Zakat Fitrah - wajib setiap Muslim bulan Ramadan / obligatory for every Muslim in Ramadan\n"
            "2. Zakat Pendapatan - 2.5% dari pendapatan bersih tahunan / 2.5% of annual net income\n"
            "3. Zakat Simpanan - 2.5% dari simpanan yang cukup nisab & haul / 2.5% of savings meeting nisab & haul\n"
            "4. Zakat Emas - 2.5% dari emas yang cukup nisab (85g) / 2.5% of gold meeting nisab (85g)\n"
            "5. Zakat Perniagaan - 2.5% dari aset bersih perniagaan / 2.5% of net business assets\n"
            "6. Zakat Pertanian - 5% (pengairan) atau 10% (tadahan hujan) / 5% irrigated or 10% rain-fed\n"
            "7. Zakat Ternakan - berdasarkan nisab jenis haiwan / based on livestock type nisab\n\n"
            "Guna type=fitrah/pendapatan/simpanan/emas/perniagaan/pertanian/ternakan untuk pengiraan.\n"
            "Use type=info_nisab untuk melihat nilai nisab semasa. / Use type=info_nisab for current nisab values."
        )

    if zakat_type == "info_nisab":
        gold_price = args.get("gold_price_per_gram", DEFAULT_GOLD_PRICE_PER_GRAM)
        if not isinstance(gold_price, (int, float)) or gold_price <= 0:
            gold_price = DEFAULT_GOLD_PRICE_PER_GRAM
        nisab_gold = NISAB_GOLD_GRAMS * gold_price
        silver_grams = 595.0
        silver_price = gold_price * 0.012  # rough estimate
        nisab_silver = silver_grams * silver_price
        return SkillResult.ok(
            "Nilai Nisab Semasa / Current Nisab Values\n\n"
            f"Harga emas / Gold price: RM {gold_price:.2f}/gram\n"
            f"(Boleh override dengan parameter gold_price_per_gram)\n\n"
            f"== Nisab Emas / Gold Nisab ==\n"
            f"  85 gram emas = RM {nisab_gold:,.2f}\n\n"
            f"== Nisab Wang & Simpanan / Cash & Savings ==\n"
            f"  Sama dengan nisab emas = RM {nisab_gold:,.2f}\n\n"
            f"== Nisab Pertanian / Agriculture ==\n"
            f"  5 wasaq = ~653 kg (padi/beras)\n\n"
            f"== Nisab Ternakan / Livestock ==\n"
            f"  Kambing / Goat: 40 ekor\n"
            f"  Lembu / Cattle: 30 ekor\n"
            f"  Unta / Camel: 5 ekor\n\n"
            f"Nota: Harga emas adalah anggaran. Semak harga semasa di LBMA/Habib Jewels.\n"
            f"Note: Gold price is estimated. Check current price from LBMA/local dealers."
        )

    if zakat_type == "fitrah":
        state = args.get("state", "WLY").upper()
        dependents = args.get("dependents", 1)
        if not isinstance(dependents, int) or dependents < 1:
            dependents = 1
        rate = ZAKAT_FITRAH_RATES.get(state, ZAKAT_FITRAH_RATES["default"])
        total = rate * dependents
        return SkillResult.ok(
            f"Zakat Fitrah ({state}):\n\n"
            f"Kadar / Rate: RM {rate:.2f} seorang / per person\n"
            f"Bilangan tanggungan / Dependents: {dependents}\n"
            f"Jumlah / Total: RM {total:.2f}"
        )

    if zakat_type == "pendapatan":
        income = args.get("annual_income", 0)
        expenses = args.get("annual_expenses", 0)
        if not isinstance(income, (int, float)) or income <= 0:
            return SkillResult.err(
                "Sila masukkan pendapatan tahunan (annual_income). / "
                "Please provide annual income."
            )
        if not isinstance(expenses, (int, float)):
            expenses = 0
        net = income - expenses
        zakat = max(0, net * 0.025)
        monthly = zakat / 12
        return SkillResult.ok(
            f"Zakat Pendapatan / Income Zakat:\n\n"
            f"Pendapatan tahunan / Annual income: RM {income:,.2f}\n"
            f"Tolakan / Deductions: RM {expenses:,.2f}\n"
            f"Pendapatan bersih / Net income: RM {net:,.2f}\n"
            f"Zakat (2.5%): RM {zakat:,.2f}\n"
            f"Anggaran bulanan / Monthly estimate: RM {monthly:,.2f}"
        )

    if zakat_type == "simpanan":
        balance = args.get("savings_balance", 0)
        if not isinstance(balance, (int, float)) or balance <= 0:
            return SkillResult.err(
                "Sila masukkan baki simpanan terendah dalam setahun (savings_balance). / "
                "Please provide lowest savings balance in the year."
            )
        gold_price = args.get("gold_price_per_gram", DEFAULT_GOLD_PRICE_PER_GRAM)
        if not isinstance(gold_price, (int, float)) or gold_price <= 0:
            gold_price = DEFAULT_GOLD_PRICE_PER_GRAM
        nisab = NISAB_GOLD_GRAMS * gold_price
        if balance < nisab:
            return SkillResult.ok(
                f"Zakat Simpanan / Savings Zakat:\n\n"
                f"Baki terendah / Lowest balance: RM {balance:,.2f}\n"
                f"Nisab (85g emas @ RM{gold_price:.2f}/g): RM {nisab:,.2f}\n"
                f"Status: TIDAK WAJIB / NOT OBLIGATORY (baki < nisab)"
            )
        zakat = balance * 0.025
        return SkillResult.ok(
            f"Zakat Simpanan / Savings Zakat:\n\n"
            f"Baki terendah / Lowest balance: RM {balance:,.2f}\n"
            f"Nisab: RM {nisab:,.2f}\n"
            f"Status: WAJIB / OBLIGATORY\n"
            f"Zakat (2.5%): RM {zakat:,.2f}"
        )

    if zakat_type == "emas":
        grams = args.get("gold_grams", 0)
        price = args.get("gold_price_per_gram", DEFAULT_GOLD_PRICE_PER_GRAM)
        if not isinstance(grams, (int, float)) or grams <= 0:
            return SkillResult.err(
                "Sila masukkan berat emas dalam gram (gold_grams). / "
                "Please provide gold weight in grams."
            )
        if not isinstance(price, (int, float)) or price <= 0:
            price = DEFAULT_GOLD_PRICE_PER_GRAM
        value = grams * price
        if grams < NISAB_GOLD_GRAMS:
            return SkillResult.ok(
                f"Zakat Emas / Gold Zakat:\n\n"
                f"Berat / Weight: {grams:.1f}g\n"
                f"Nisab: {NISAB_GOLD_GRAMS:.0f}g\n"
                f"Status: TIDAK WAJIB / NOT OBLIGATORY (berat < nisab)"
            )
        zakat = value * 0.025
        return SkillResult.ok(
            f"Zakat Emas / Gold Zakat:\n\n"
            f"Berat / Weight: {grams:.1f}g @ RM{price:.2f}/g\n"
            f"Nilai / Value: RM {value:,.2f}\n"
            f"Zakat (2.5%): RM {zakat:,.2f}"
        )

    if zakat_type == "perniagaan":
        net_assets = args.get("net_business_assets", 0)
        if not isinstance(net_assets, (int, float)) or net_assets <= 0:
            return SkillResult.err(
                "Sila masukkan aset bersih perniagaan (net_business_assets). / "
                "Please provide net business assets (current assets - current liabilities)."
            )
        gold_price = args.get("gold_price_per_gram", DEFAULT_GOLD_PRICE_PER_GRAM)
        if not isinstance(gold_price, (int, float)) or gold_price <= 0:
            gold_price = DEFAULT_GOLD_PRICE_PER_GRAM
        nisab = NISAB_GOLD_GRAMS * gold_price
        if net_assets < nisab:
            return SkillResult.ok(
                f"Zakat Perniagaan / Business Zakat:\n\n"
                f"Aset bersih / Net assets: RM {net_assets:,.2f}\n"
                f"Nisab (85g emas @ RM{gold_price:.2f}/g): RM {nisab:,.2f}\n"
                f"Status: TIDAK WAJIB / NOT OBLIGATORY (aset < nisab)"
            )
        zakat = net_assets * 0.025
        return SkillResult.ok(
            f"Zakat Perniagaan / Business Zakat:\n\n"
            f"Aset bersih / Net assets: RM {net_assets:,.2f}\n"
            f"Nisab: RM {nisab:,.2f}\n"
            f"Status: WAJIB / OBLIGATORY\n"
            f"Zakat (2.5%): RM {zakat:,.2f}\n\n"
            f"Formula: (Aset semasa - Liabiliti semasa) x 2.5%\n"
            f"Formula: (Current assets - Current liabilities) x 2.5%"
        )

    if zakat_type == "pertanian":
        harvest = args.get("harvest_kg", 0)
        irrigation = args.get("irrigation", "rain")
        if not isinstance(harvest, (int, float)) or harvest <= 0:
            return SkillResult.err(
                "Sila masukkan berat hasil tuaian dalam kg (harvest_kg). / "
                "Please provide harvest weight in kg."
            )
        nisab_kg = 653.0  # ~5 wasaq
        if harvest < nisab_kg:
            return SkillResult.ok(
                f"Zakat Pertanian / Agriculture Zakat:\n\n"
                f"Hasil tuaian / Harvest: {harvest:,.1f} kg\n"
                f"Nisab: {nisab_kg:,.0f} kg (5 wasaq)\n"
                f"Status: TIDAK WAJIB / NOT OBLIGATORY (hasil < nisab)"
            )
        rates = {"rain": 10.0, "irrigated": 5.0, "mixed": 7.5}
        rate = rates.get(irrigation, 10.0)
        labels = {"rain": "tadahan hujan / rain-fed", "irrigated": "pengairan / irrigated", "mixed": "campuran / mixed"}
        label = labels.get(irrigation, "tadahan hujan / rain-fed")
        zakat_kg = harvest * (rate / 100.0)
        return SkillResult.ok(
            f"Zakat Pertanian / Agriculture Zakat:\n\n"
            f"Hasil tuaian / Harvest: {harvest:,.1f} kg\n"
            f"Nisab: {nisab_kg:,.0f} kg (5 wasaq)\n"
            f"Jenis pengairan / Irrigation: {label}\n"
            f"Kadar / Rate: {rate:.1f}%\n"
            f"Status: WAJIB / OBLIGATORY\n"
            f"Zakat: {zakat_kg:,.1f} kg\n\n"
            f"Nota: 10% jika tadahan hujan, 5% jika pengairan, 7.5% jika campuran.\n"
            f"Note: 10% rain-fed, 5% irrigated, 7.5% mixed."
        )

    if zakat_type == "ternakan":
        ltype = args.get("livestock_type", "goat")
        count = args.get("livestock_count", 0)
        if not isinstance(count, (int, float)) or count <= 0:
            return SkillResult.err(
                "Sila masukkan bilangan ternakan (livestock_count). / "
                "Please provide livestock count."
            )
        count = int(count)
        if ltype not in LIVESTOCK_NISAB:
            return SkillResult.err(
                f"Jenis ternakan tidak dikenali: {ltype}. "
                f"Pilih: goat, cattle, camel."
            )
        info = LIVESTOCK_NISAB[ltype]
        nisab = info["nisab"]
        type_labels = {"goat": "Kambing / Goat", "cattle": "Lembu / Cattle", "camel": "Unta / Camel"}
        label = type_labels[ltype]
        if count < nisab:
            return SkillResult.ok(
                f"Zakat Ternakan / Livestock Zakat ({label}):\n\n"
                f"Bilangan / Count: {count}\n"
                f"Nisab: {nisab} ekor\n"
                f"Status: TIDAK WAJIB / NOT OBLIGATORY (bilangan < nisab)"
            )
        # Calculate zakat based on type
        if ltype == "goat":
            zakat_table = _goat_zakat(count)
        elif ltype == "cattle":
            zakat_table = _cattle_zakat(count)
        else:
            zakat_table = _camel_zakat(count)
        return SkillResult.ok(
            f"Zakat Ternakan / Livestock Zakat ({label}):\n\n"
            f"Bilangan / Count: {count}\n"
            f"Nisab: {nisab} ekor\n"
            f"Status: WAJIB / OBLIGATORY\n"
            f"Zakat: {zakat_table}"
        )

    return SkillResult.err(
        f"Jenis zakat tidak dikenali / Unknown zakat type: {zakat_type}"
    )


def _goat_zakat(count: int) -> str:
    """Simplified goat zakat table."""
    if count < 40:
        return "Tidak wajib / Not obligatory"
    elif count <= 120:
        return "1 ekor kambing (1 goat)"
    elif count <= 200:
        return "2 ekor kambing (2 goats)"
    elif count <= 399:
        return "3 ekor kambing (3 goats)"
    else:
        n = count // 100
        return f"{n} ekor kambing ({n} goats) — 1 ekor per 100"


def _cattle_zakat(count: int) -> str:
    """Simplified cattle zakat table."""
    if count < 30:
        return "Tidak wajib / Not obligatory"
    elif count <= 39:
        return "1 ekor tabi' / 1 young cattle (1 year old)"
    elif count <= 59:
        return "1 ekor musinnah / 1 mature cattle (2 years old)"
    elif count <= 69:
        return "2 ekor tabi' / 2 young cattle"
    elif count <= 79:
        return "1 tabi' + 1 musinnah"
    else:
        tabi = count // 30
        return f"{tabi} ekor tabi' ({tabi} young cattle) — 1 per 30 ekor"


def _camel_zakat(count: int) -> str:
    """Simplified camel zakat table."""
    if count < 5:
        return "Tidak wajib / Not obligatory"
    elif count <= 9:
        return "1 ekor kambing (1 goat)"
    elif count <= 14:
        return "2 ekor kambing (2 goats)"
    elif count <= 19:
        return "3 ekor kambing (3 goats)"
    elif count <= 24:
        return "4 ekor kambing (4 goats)"
    elif count <= 35:
        return "1 ekor bintu makhad (1 female camel, 1 year)"
    elif count <= 45:
        return "1 ekor bintu labun (1 female camel, 2 years)"
    elif count <= 60:
        return "1 ekor hiqqah (1 female camel, 3 years)"
    elif count <= 75:
        return "1 ekor jadz'ah (1 female camel, 4 years)"
    else:
        return f"Rujuk jadual penuh zakat unta / Refer to full camel zakat table"


if __name__ == "__main__":
    execute.run()
