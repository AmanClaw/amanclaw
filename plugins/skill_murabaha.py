#!/usr/bin/env python3
"""AmanClaw Plugin: Islamic Financing Calculator (Murabaha).

Compare Murabaha (cost-plus), Musharakah Mutanaqisah (diminishing
partnership), and Ijarah (lease) Islamic financing options.
"""

from amanclaw_sdk import plugin, SkillInput, SkillResult


@plugin(
    name="murabaha",
    description=(
        "Islamic financing calculator. Compare Murabaha (cost-plus), "
        "Musharakah Mutanaqisah (diminishing partnership), and Ijarah "
        "(lease) financing options. Kalkulator pembiayaan Islam."
    ),
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["calculate", "compare", "explain"],
                "description": "Operation: calculate, compare, or explain financing types",
            },
            "property_price": {
                "type": "number",
                "description": "Property/asset price (RM)",
            },
            "down_payment": {
                "type": "number",
                "description": "Down payment amount (RM)",
            },
            "profit_rate": {
                "type": "number",
                "description": "Bank's profit rate per annum as percentage (e.g. 4.5 for 4.5%)",
            },
            "tenure_years": {
                "type": "integer",
                "description": "Financing tenure in years",
            },
            "financing_type": {
                "type": "string",
                "enum": ["murabaha", "musharakah", "ijarah"],
                "description": "Type of Islamic financing",
            },
        },
        "required": ["action"],
    },
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "explain")

    if action == "explain":
        return _explain()
    elif action == "calculate":
        return _calculate(args)
    elif action == "compare":
        return _compare(args)
    else:
        return SkillResult.err(f"Unknown action: {action}")


def _explain() -> SkillResult:
    return SkillResult.ok(
        "Pembiayaan Islam vs Pinjaman Konvensional\n"
        "Islamic Financing vs Conventional Loans\n\n"
        "== Perbezaan Utama / Key Differences ==\n\n"
        "Konvensional: Bank meminjamkan wang dan mengenakan faedah (riba).\n"
        "Conventional: Bank lends money and charges interest (riba).\n\n"
        "Islam: Bank membeli aset dan menjual/menyewa kepada pelanggan.\n"
        "Islamic: Bank buys asset and sells/leases to customer.\n\n"
        "== Jenis Pembiayaan Islam / Types of Islamic Financing ==\n\n"
        "1. MURABAHA (Kos Tambah / Cost-Plus)\n"
        "   Bank membeli aset, kemudian menjual kepada pelanggan pada\n"
        "   harga kos + margin keuntungan tetap. Ansuran bulanan tetap.\n"
        "   Bank buys asset, then sells to customer at cost + fixed\n"
        "   profit margin. Fixed monthly instalments.\n"
        "   Pros: Predictable payments, simple structure\n"
        "   Cons: Total cost is fixed upfront, no benefit from rate drops\n\n"
        "2. MUSHARAKAH MUTANAQISAH (Perkongsian Berkurangan / Diminishing Partnership)\n"
        "   Bank dan pelanggan sama-sama memiliki aset. Pelanggan membeli\n"
        "   bahagian bank secara beransur + bayar sewa bahagian bank.\n"
        "   Bank and customer co-own asset. Customer gradually buys\n"
        "   bank's share + pays rent on bank's portion.\n"
        "   Pros: Customer's equity grows, fairer structure\n"
        "   Cons: More complex, rental may vary\n\n"
        "3. IJARAH (Sewaan / Lease)\n"
        "   Bank membeli aset dan menyewa kepada pelanggan. Di akhir\n"
        "   tempoh, pelanggan boleh membeli aset pada harga nominal.\n"
        "   Bank buys asset and leases to customer. At end of tenure,\n"
        "   customer may purchase at nominal price.\n"
        "   Pros: Flexibility, maintenance may be bank's responsibility\n"
        "   Cons: No equity until purchase, total cost may be higher\n\n"
        "Guna action=calculate atau action=compare untuk pengiraan.\n"
        "Use action=calculate or action=compare for calculations."
    )


def _validate_inputs(args: dict) -> str | None:
    """Return error message if inputs are invalid, else None."""
    price = args.get("property_price")
    if price is None or not isinstance(price, (int, float)) or price <= 0:
        return (
            "Sila masukkan harga hartanah (property_price). / "
            "Please provide property/asset price."
        )
    dp = args.get("down_payment", 0)
    if not isinstance(dp, (int, float)) or dp < 0:
        return "Down payment tidak sah. / Invalid down payment."
    if dp >= price:
        return "Down payment mesti kurang dari harga. / Down payment must be less than price."
    rate = args.get("profit_rate")
    if rate is None or not isinstance(rate, (int, float)) or rate <= 0:
        return (
            "Sila masukkan kadar keuntungan (profit_rate, e.g. 4.5). / "
            "Please provide profit rate."
        )
    tenure = args.get("tenure_years")
    if tenure is None or not isinstance(tenure, (int, float)) or tenure < 1:
        return (
            "Sila masukkan tempoh pembiayaan dalam tahun (tenure_years). / "
            "Please provide financing tenure in years."
        )
    return None


def _calc_murabaha(price: float, dp: float, rate: float, years: int) -> dict:
    """Murabaha: cost-plus, fixed selling price."""
    financed = price - dp
    total_profit = financed * (rate / 100.0) * years
    selling_price = financed + total_profit
    months = years * 12
    monthly = selling_price / months
    return {
        "type": "Murabaha (Kos Tambah / Cost-Plus)",
        "financed": financed,
        "total_profit": total_profit,
        "selling_price": selling_price,
        "monthly": monthly,
        "total_paid": selling_price + dp,
        "months": months,
    }


def _calc_musharakah(price: float, dp: float, rate: float, years: int) -> dict:
    """Musharakah Mutanaqisah: diminishing partnership with declining balance."""
    financed = price - dp
    months = years * 12
    monthly_rate = rate / 100.0 / 12.0
    # Similar to reducing-balance: monthly payment = P * r / (1 - (1+r)^-n)
    if monthly_rate > 0:
        monthly = financed * monthly_rate / (1 - (1 + monthly_rate) ** (-months))
    else:
        monthly = financed / months
    total_paid_financing = monthly * months
    total_profit = total_paid_financing - financed
    return {
        "type": "Musharakah Mutanaqisah (Perkongsian Berkurangan / Diminishing Partnership)",
        "financed": financed,
        "total_profit": total_profit,
        "selling_price": total_paid_financing,
        "monthly": monthly,
        "total_paid": total_paid_financing + dp,
        "months": months,
    }


def _calc_ijarah(price: float, dp: float, rate: float, years: int) -> dict:
    """Ijarah: lease-to-own with fixed rental."""
    financed = price - dp
    months = years * 12
    # Rental covers profit; at end, asset transferred at nominal price
    total_rental = financed * (rate / 100.0) * years
    # Principal repayment spread over tenure
    monthly_principal = financed / months
    monthly_rental = total_rental / months
    monthly = monthly_principal + monthly_rental
    total_paid_financing = monthly * months
    return {
        "type": "Ijarah (Sewaan / Lease-to-Own)",
        "financed": financed,
        "total_profit": total_rental,
        "selling_price": total_paid_financing,
        "monthly": monthly,
        "total_paid": total_paid_financing + dp,
        "months": months,
    }


def _format_result(r: dict, price: float, dp: float, rate: float, years: int) -> str:
    """Format a single calculation result."""
    return (
        f"Jenis / Type: {r['type']}\n"
        f"Harga aset / Asset price: RM {price:,.2f}\n"
        f"Wang pendahuluan / Down payment: RM {dp:,.2f}\n"
        f"Jumlah dibiayai / Financed: RM {r['financed']:,.2f}\n"
        f"Kadar keuntungan / Profit rate: {rate:.2f}% p.a.\n"
        f"Tempoh / Tenure: {years} tahun ({r['months']} bulan)\n\n"
        f"Ansuran bulanan / Monthly payment: RM {r['monthly']:,.2f}\n"
        f"Jumlah keuntungan bank / Total bank profit: RM {r['total_profit']:,.2f}\n"
        f"Jumlah bayaran / Total payment: RM {r['total_paid']:,.2f}"
    )


def _calculate(args: dict) -> SkillResult:
    err = _validate_inputs(args)
    if err:
        return SkillResult.err(err)

    price = args["property_price"]
    dp = args.get("down_payment", 0)
    rate = args["profit_rate"]
    years = int(args["tenure_years"])
    ftype = args.get("financing_type", "murabaha")

    calculators = {
        "murabaha": _calc_murabaha,
        "musharakah": _calc_musharakah,
        "ijarah": _calc_ijarah,
    }

    calc = calculators.get(ftype)
    if not calc:
        return SkillResult.err(
            f"Jenis pembiayaan tidak dikenali: {ftype}. "
            f"Pilih: murabaha, musharakah, ijarah."
        )

    result = calc(price, dp, rate, years)
    return SkillResult.ok(
        "Pengiraan Pembiayaan Islam / Islamic Financing Calculation\n\n"
        + _format_result(result, price, dp, rate, years)
    )


def _compare(args: dict) -> SkillResult:
    err = _validate_inputs(args)
    if err:
        return SkillResult.err(err)

    price = args["property_price"]
    dp = args.get("down_payment", 0)
    rate = args["profit_rate"]
    years = int(args["tenure_years"])

    m = _calc_murabaha(price, dp, rate, years)
    s = _calc_musharakah(price, dp, rate, years)
    j = _calc_ijarah(price, dp, rate, years)

    lines = [
        "Perbandingan Pembiayaan Islam / Islamic Financing Comparison\n",
        f"Harga aset / Asset price: RM {price:,.2f}",
        f"Wang pendahuluan / Down payment: RM {dp:,.2f}",
        f"Jumlah dibiayai / Financed: RM {m['financed']:,.2f}",
        f"Kadar / Rate: {rate:.2f}% p.a. | Tempoh / Tenure: {years} tahun\n",
        f"{'':30s} {'Murabaha':>15s} {'Musharakah':>15s} {'Ijarah':>15s}",
        f"{'':30s} {'(Kos Tambah)':>15s} {'(Berkurangan)':>15s} {'(Sewaan)':>15s}",
        "-" * 78,
        f"{'Ansuran bulanan / Monthly':30s} RM {m['monthly']:>11,.2f} RM {s['monthly']:>11,.2f} RM {j['monthly']:>11,.2f}",
        f"{'Keuntungan bank / Profit':30s} RM {m['total_profit']:>11,.2f} RM {s['total_profit']:>11,.2f} RM {j['total_profit']:>11,.2f}",
        f"{'Jumlah bayaran / Total':30s} RM {m['total_paid']:>11,.2f} RM {s['total_paid']:>11,.2f} RM {j['total_paid']:>11,.2f}",
        "",
        "Nota / Notes:",
        "- Murabaha: Harga tetap, ansuran tetap. Paling mudah difahami.",
        "  Fixed price, fixed instalments. Simplest to understand.",
        "- Musharakah: Baki berkurangan, biasanya jumlah lebih rendah.",
        "  Declining balance, usually lower total cost.",
        "- Ijarah: Berasaskan sewa, fleksibel tetapi jumlah mungkin lebih tinggi.",
        "  Rental-based, flexible but total may be higher.",
    ]

    # Find best option
    options = [("Murabaha", m), ("Musharakah", s), ("Ijarah", j)]
    best = min(options, key=lambda x: x[1]["total_paid"])
    lines.append(f"\nPilihan terbaik / Best option: {best[0]} (jumlah terendah / lowest total)")

    return SkillResult.ok("\n".join(lines))


if __name__ == "__main__":
    execute.run()
