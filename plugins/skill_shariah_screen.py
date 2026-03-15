#!/usr/bin/env python3
"""AmanClaw Plugin: Shariah Stock Screening.

Check if stocks/investments are Shariah-compliant based on standard
screening criteria (AAOIFI / Securities Commission Malaysia).
"""

from amanclaw_sdk import plugin, SkillInput, SkillResult

# Thresholds based on SC Malaysia & AAOIFI guidelines
DEBT_RATIO_THRESHOLD = 0.33       # Total debt / total assets < 33%
NON_HALAL_REV_THRESHOLD = 0.05    # Non-halal revenue / total revenue < 5%
CASH_INTEREST_THRESHOLD = 0.33    # Cash + interest-bearing / total assets < 33%


@plugin(
    name="shariah_screen",
    description=(
        "Check if stocks/investments are Shariah-compliant. "
        "Screens based on debt ratio, revenue sources, and purification calculations. "
        "Semak sama ada saham/pelaburan patuh Syariah."
    ),
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["screen", "criteria", "purify"],
                "description": "Operation: screen a stock, show criteria, or calculate purification",
            },
            "company": {
                "type": "string",
                "description": "Company or stock name to screen",
            },
            "total_debt": {
                "type": "number",
                "description": "Total debt/borrowing (RM)",
            },
            "total_assets": {
                "type": "number",
                "description": "Total assets or market capitalisation (RM)",
            },
            "non_halal_revenue": {
                "type": "number",
                "description": "Revenue from non-halal sources (RM)",
            },
            "total_revenue": {
                "type": "number",
                "description": "Total revenue (RM)",
            },
            "cash_and_interest": {
                "type": "number",
                "description": "Cash and interest-bearing securities (RM)",
            },
            "dividend_per_share": {
                "type": "number",
                "description": "Dividend per share (RM) for purification",
            },
            "non_halal_ratio": {
                "type": "number",
                "description": "Non-halal income ratio as percentage (e.g. 3.5) for purification",
            },
            "shares_held": {
                "type": "integer",
                "description": "Number of shares held for purification calculation",
            },
        },
        "required": ["action"],
    },
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "criteria")

    if action == "criteria":
        return _show_criteria()
    elif action == "screen":
        return _screen_stock(args)
    elif action == "purify":
        return _calculate_purification(args)
    else:
        return SkillResult.err(f"Unknown action: {action}")


def _show_criteria() -> SkillResult:
    return SkillResult.ok(
        "Kriteria Saringan Syariah / Shariah Screening Criteria\n"
        "(Based on SC Malaysia & AAOIFI Standards)\n\n"
        "== Quantitative Benchmarks ==\n\n"
        "1. Debt Ratio (Nisbah Hutang)\n"
        "   Total debt / Total assets < 33%\n"
        "   Hutang keseluruhan / Jumlah aset < 33%\n\n"
        "2. Non-Halal Revenue (Pendapatan Tidak Halal)\n"
        "   Non-halal revenue / Total revenue < 5%\n"
        "   Pendapatan tidak halal / Jumlah pendapatan < 5%\n"
        "   Sources: conventional banking, gambling, alcohol, tobacco, pork, etc.\n\n"
        "3. Cash & Interest-Bearing Securities\n"
        "   Cash + interest-bearing / Total assets < 33%\n\n"
        "== Qualitative Criteria ==\n\n"
        "- Core business must be halal\n"
        "  (Perniagaan teras mestilah halal)\n"
        "- No involvement in: gambling (judi), alcohol (arak),\n"
        "  pork-related (khinzir), conventional finance (riba),\n"
        "  tobacco, weapons, adult entertainment\n\n"
        "== Purification (Pembersihan) ==\n\n"
        "If a Shariah-compliant stock earns minor non-halal income,\n"
        "investors must purify dividends by donating:\n"
        "  Purification = Dividend x (Non-halal income / Total income)\n"
        "Donate to charity — not considered zakat."
    )


def _screen_stock(args: dict) -> SkillResult:
    company = args.get("company", "Unknown")
    total_debt = args.get("total_debt")
    total_assets = args.get("total_assets")
    non_halal_rev = args.get("non_halal_revenue")
    total_rev = args.get("total_revenue")
    cash_interest = args.get("cash_and_interest")

    if total_assets is None or not isinstance(total_assets, (int, float)) or total_assets <= 0:
        return SkillResult.err(
            "Sila masukkan jumlah aset (total_assets). / "
            "Please provide total assets."
        )

    lines = [f"Saringan Syariah / Shariah Screening: {company}\n"]
    results = []
    all_pass = True

    # 1. Debt ratio
    if total_debt is not None and isinstance(total_debt, (int, float)):
        debt_ratio = total_debt / total_assets
        passed = debt_ratio < DEBT_RATIO_THRESHOLD
        status = "LULUS / PASS" if passed else "GAGAL / FAIL"
        if not passed:
            all_pass = False
        lines.append(
            f"1. Debt Ratio: {debt_ratio:.1%} (threshold < 33%) — {status}"
        )
        results.append(("debt", passed))
    else:
        lines.append("1. Debt Ratio: tidak diberikan / not provided — SKIP")

    # 2. Non-halal revenue
    if (non_halal_rev is not None and isinstance(non_halal_rev, (int, float))
            and total_rev is not None and isinstance(total_rev, (int, float))
            and total_rev > 0):
        nh_ratio = non_halal_rev / total_rev
        passed = nh_ratio < NON_HALAL_REV_THRESHOLD
        status = "LULUS / PASS" if passed else "GAGAL / FAIL"
        if not passed:
            all_pass = False
        lines.append(
            f"2. Non-Halal Revenue: {nh_ratio:.2%} (threshold < 5%) — {status}"
        )
        results.append(("non_halal", passed))
    else:
        lines.append("2. Non-Halal Revenue: tidak diberikan / not provided — SKIP")

    # 3. Cash & interest-bearing
    if cash_interest is not None and isinstance(cash_interest, (int, float)):
        ci_ratio = cash_interest / total_assets
        passed = ci_ratio < CASH_INTEREST_THRESHOLD
        status = "LULUS / PASS" if passed else "GAGAL / FAIL"
        if not passed:
            all_pass = False
        lines.append(
            f"3. Cash & Interest: {ci_ratio:.1%} (threshold < 33%) — {status}"
        )
        results.append(("cash_interest", passed))
    else:
        lines.append("3. Cash & Interest: tidak diberikan / not provided — SKIP")

    # Overall verdict
    if not results:
        lines.append(
            "\nKeputusan / Result: TIDAK DAPAT DITENTUKAN / INSUFFICIENT DATA\n"
            "Sila berikan sekurang-kurangnya satu nisbah kewangan. / "
            "Please provide at least one financial ratio."
        )
    elif all_pass:
        lines.append(
            "\nKeputusan / Result: PATUH SYARIAH / SHARIAH COMPLIANT\n"
            "Semua nisbah di bawah ambang. / All ratios below thresholds."
        )
    else:
        failed = [name for name, p in results if not p]
        lines.append(
            f"\nKeputusan / Result: TIDAK PATUH SYARIAH / NON-COMPLIANT\n"
            f"Gagal pada / Failed on: {', '.join(failed)}"
        )

    lines.append(
        "\nNota: Saringan ini berdasarkan data yang diberikan sahaja. "
        "Rujuk senarai rasmi SC Malaysia untuk status terkini.\n"
        "Note: This screening is based on provided data only. "
        "Refer to official SC Malaysia list for current status."
    )

    return SkillResult.ok("\n".join(lines))


def _calculate_purification(args: dict) -> SkillResult:
    dps = args.get("dividend_per_share")
    nh_ratio = args.get("non_halal_ratio")
    shares = args.get("shares_held", 1)

    if dps is None or not isinstance(dps, (int, float)):
        return SkillResult.err(
            "Sila masukkan dividen sesaham (dividend_per_share). / "
            "Please provide dividend per share."
        )
    if nh_ratio is None or not isinstance(nh_ratio, (int, float)):
        return SkillResult.err(
            "Sila masukkan nisbah pendapatan tidak halal sebagai peratusan "
            "(non_halal_ratio, e.g. 3.5 untuk 3.5%). / "
            "Please provide non-halal income ratio as percentage."
        )
    if not isinstance(shares, (int, float)) or shares < 1:
        shares = 1

    ratio_decimal = nh_ratio / 100.0
    purify_per_share = dps * ratio_decimal
    total_dividend = dps * shares
    total_purify = purify_per_share * shares

    return SkillResult.ok(
        "Pengiraan Pembersihan Dividen / Dividend Purification\n\n"
        f"Dividen sesaham / Dividend per share: RM {dps:.4f}\n"
        f"Nisbah tidak halal / Non-halal ratio: {nh_ratio:.2f}%\n"
        f"Saham dipegang / Shares held: {int(shares):,}\n\n"
        f"Pembersihan sesaham / Purification per share: RM {purify_per_share:.4f}\n"
        f"Jumlah dividen / Total dividend: RM {total_dividend:,.2f}\n"
        f"Jumlah pembersihan / Total purification: RM {total_purify:,.2f}\n\n"
        "Amaun pembersihan perlu disedekahkan, bukan dikira sebagai zakat.\n"
        "Purification amount should be donated to charity, not counted as zakat."
    )


if __name__ == "__main__":
    execute.run()
