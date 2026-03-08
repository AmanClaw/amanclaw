#!/usr/bin/env python3
"""AmanClaw Plugin: Zakat Calculator.

Calculate zakat (Islamic tax): fitrah, pendapatan (income),
simpanan (savings), and emas (gold).
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


@plugin(
    name="zakat",
    description="Calculate zakat (Islamic tax). Supports zakat fitrah, zakat pendapatan (income), zakat simpanan (savings), and zakat emas (gold). Kira zakat fitrah, pendapatan, simpanan, dan emas.",
    parameters={
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "enum": ["fitrah", "pendapatan", "simpanan", "emas", "info"],
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
                "description": "Current gold price per gram (RM)",
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
            "4. Zakat Emas - 2.5% dari emas yang cukup nisab (85g) / 2.5% of gold meeting nisab (85g)\n\n"
            "Guna type=fitrah/pendapatan/simpanan/emas untuk pengiraan. / Use type= to calculate."
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
        gold_price = args.get("gold_price_per_gram", 400.0)
        if not isinstance(gold_price, (int, float)) or gold_price <= 0:
            gold_price = 400.0
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
        price = args.get("gold_price_per_gram", 400.0)
        if not isinstance(grams, (int, float)) or grams <= 0:
            return SkillResult.err(
                "Sila masukkan berat emas dalam gram (gold_grams). / "
                "Please provide gold weight in grams."
            )
        if not isinstance(price, (int, float)) or price <= 0:
            price = 400.0
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

    return SkillResult.err(
        f"Jenis zakat tidak dikenali / Unknown zakat type: {zakat_type}"
    )


if __name__ == "__main__":
    execute.run()
