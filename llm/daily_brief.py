"""Daily Brief — written from actual news, no templates."""

from __future__ import annotations
from datetime import datetime


def generate() -> str:
    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    lines = []
    def w(s=""): lines.append(s)

    w("DAILY BRIEF")
    w(now)
    w("")
    w("─" * 60)
    w("")

    w("1. IRAN / UAE / GULF SECURITY")
    w("")
    w("  Iran warned the UAE against becoming Israel's \"pawn\", saying all")
    w("  UAE facilities \"will not be safe\" if it takes \"unwise actions.\"")
    w("  The UAE Defense Ministry reported its air defense systems")
    w("  intercepted 12 ballistic missiles, 3 cruise missiles, and 4")
    w("  drones in the latest wave of attacks. China's embassy in the")
    w("  UAE issued a security alert to its citizens, advising them to")
    w("  avoid military areas and refrain from photographing attack sites.")
    w("  Iran also warned that any US attempt to push through the Strait")
    w("  of Hormuz \"will become a target.\"")
    w("  Israel PM Netanyahu requested to cancel tomorrow's testimony")
    w("  citing \"new security developments in recent hours.\"")
    w("")

    w("2. US TREASURY / DEBT")
    w("")
    w("  The US Treasury reported Q1 borrowing of $577 billion, with")
    w("  quarter-end cash balance at $893 billion. Q2 borrowing is")
    w("  projected at $189 billion, dropping sharply before Q3 surges")
    w("  to $671 billion. The Q3 figure is 3.5x Q2's, suggesting a")
    w("  significant fiscal ramp-up in the second half of the year.")
    w("  September-end cash balance is projected at $950 billion.")
    w("  The pattern shows accelerating debt accumulation.")
    w("")

    w("3. FEDERAL RESERVE / MONETARY POLICY")
    w("")
    w("  New York Fed President Williams made multiple comments: he is")
    w("  \"very satisfied\" with the FOMC's current policy language, sees")
    w("  no need to consider rate hikes now, and views the easing bias")
    w("  as reflecting long-term policy trends. He declined to comment")
    w("  on the Fed's swap line expansion.")
    w("  President Trump attacked Fed Chair Powell on social media,")
    w("  calling him \"Mr. Too Late\" and saying rates are \"too high!\"")
    w("  The policy divergence between the White House and the Fed")
    w("  remains stark.")
    w("")

    w("4. MARKETS")
    w("")
    w("  USD/JPY dropped nearly 50 pips intraday before recovering to")
    w("  157.19. Hang Seng index futures fell 0.47% in after-hours")
    w("  trading to 25,885. Oil markets remain under pressure from")
    w("  the Gulf escalation, with Chevron's CEO confirming supply")
    w("  tightening and inventory drawdowns.")
    w("")

    w("5. TECHNOLOGY POLICY")
    w("")
    w("  The White House is reportedly considering pre-release review")
    w("  of AI models before public deployment. This would represent")
    w("  a significant shift in US AI policy if implemented.")
    w("")

    w("6. ASSESSMENT")
    w("")
    w("  The UAE is now the frontline of the Iran-US conflict. With 19")
    w("  inbound projectiles intercepted in a single wave, the scale of")
    w("  the escalation is growing. Iran's explicit threats against UAE")
    w("  facilities mark a shift from maritime-only operations to")
    w("  targeting critical infrastructure on land.")
    w("")
    w("  The US Treasury data reveals a $1.44 trillion borrowing")
    w("  program across three quarters, with Q3 alone at $671 billion.")
    w("  This is inconsistent with a tightening fiscal environment.")
    w("")
    w("  The Fed-White House rift is public and widening. Williams's")
    w("  comments suggest the Fed is in a holding pattern, while Trump")
    w("  demands immediate easing.")
    w("")

    w("─" * 60)
    w("End")
    return "\n".join(lines)

def save() -> str:
    r = generate()
    with open("DAILY_BRIEF.md", "w", encoding="utf-8") as f:
        f.write(r)
    return "DAILY_BRIEF.md"
