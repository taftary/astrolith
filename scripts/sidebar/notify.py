"""Sends an agent's message to the owner, tagged with its issue (#613, P4 A).

Every message starts with `#N:`, so the owner can tell agents apart in the
one Telegram chat and answer by replying to it or by starting with `#N:`.
The message is always posted on issue #N as well (the record, and where the
agent reads the owner's answers, relayed there by the main Telegram agent
through `relay.py`). Each message records its issue as the last asker, so
an owner answer naming no issue goes to that agent.
Telegram delivery uses the bot through `TELEGRAM_BOT_TOKEN` and
`TELEGRAM_CHAT_ID`; without them only the issue comment is posted.

Usage:
  python scripts/sidebar/notify.py --issue N "text"
"""
import argparse
import json
import os
import subprocess
import sys
import urllib.parse
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def tagged(issue, text):
    """`#N: text`, never tagged twice."""
    text = text.strip()
    prefix = f"#{issue}:"
    return text if text.startswith(prefix) else f"{prefix} {text}"


def last_file():
    """Where the last asking agent's issue is kept (main checkout's .agent/)."""
    import worktrees
    import development
    return worktrees.evidence_root(development.main_checkout()) / "agents" / "last-asker.json"


def remember(issue):
    """Records `issue` as the agent that asked last; tests replace this."""
    path = last_file()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"issue": issue}), encoding="utf-8")


def last_asker():
    """The issue of the agent that asked last, or None; tests replace this."""
    try:
        return int(json.loads(last_file().read_text(encoding="utf-8"))["issue"])
    except (OSError, ValueError, KeyError, TypeError):
        return None


def run(cmd):
    """Run one gh command; tests replace this with a fake runner."""
    return subprocess.run(cmd, capture_output=True, text=True,
                          encoding="utf-8", errors="replace")


def send_telegram(token, chat, text):
    """Sends through the Bot API; tests replace this. True on success."""
    data = urllib.parse.urlencode({"chat_id": chat, "text": text}).encode()
    url = f"https://api.telegram.org/bot{token}/sendMessage"
    try:
        with urllib.request.urlopen(url, data=data, timeout=20) as reply:
            return json.loads(reply.read().decode("utf-8")).get("ok", False)
    except (OSError, ValueError):
        return False


def main(argv=None):
    ap = argparse.ArgumentParser(description="Message the owner about an issue")
    ap.add_argument("--issue", type=int, required=True)
    ap.add_argument("text")
    a = ap.parse_args(argv)
    message = tagged(a.issue, a.text)
    remember(a.issue)
    posted = run(["gh", "issue", "comment", str(a.issue), "--body",
                  f"Agent message to the owner:\n\n> {message}"]).returncode == 0
    token = os.environ.get("TELEGRAM_BOT_TOKEN", "")
    chat = os.environ.get("TELEGRAM_CHAT_ID", "")
    sent = bool(token and chat) and send_telegram(token, chat, message)
    print(json.dumps({"ok": posted or sent, "issue": a.issue, "message": message,
                      "issueComment": posted, "telegram": sent,
                      "note": "" if token and chat else "TELEGRAM_BOT_TOKEN/TELEGRAM_CHAT_ID not set"}))
    return 0 if (posted or sent) else 1


if __name__ == "__main__":
    sys.exit(main())
