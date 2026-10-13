"""Routes the owner's Telegram answer to the right agent's issue (#613, P4 A).

Worker agents send their questions with `notify.py`, which records the
issue of the agent that asked last. The main Telegram agent passes each
owner answer here:
  - an answer starting with `#N:` goes to issue N;
  - `--issue N` (the owner replied to an agent's `#N:` message) goes to N;
  - an answer naming no issue goes to the agent that asked last.
The answer is posted on that issue, where the worker agent reads it; the
worker repeats the issue back (`#N: got it: ...`) when it acts on it.
JSON to stdout names the issue it went to.

Usage:
  python scripts/sidebar/relay.py [--issue N] "answer text"
"""
import argparse
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import notify  # noqa: E402

TAGGED = re.compile(r"^\s*#(\d+)\s*:\s*(.*)$", re.DOTALL)


def route(text, replied_to=None, last=None):
    """(issue, answer) for an owner answer, or (None, text) when no issue
    can be named."""
    m = TAGGED.match(text or "")
    if m:
        return int(m.group(1)), m.group(2).strip()
    if replied_to:
        return int(replied_to), (text or "").strip()
    if last:
        return int(last), (text or "").strip()
    return None, (text or "").strip()


def main(argv=None):
    ap = argparse.ArgumentParser(description="Route an owner answer to an agent's issue")
    ap.add_argument("--issue", type=int, default=None,
                    help="the issue of the agent message the owner replied to")
    ap.add_argument("text")
    a = ap.parse_args(argv)
    last = notify.last_asker()
    issue, answer = route(a.text, a.issue, last)
    if issue is None:
        print(json.dumps({"ok": False, "error": "no issue named and no agent has asked yet"}))
        return 1
    how = "tagged" if TAGGED.match(a.text or "") else ("reply" if a.issue else "last asker")
    p = notify.run(["gh", "issue", "comment", str(issue), "--body",
                    f"<!-- owner-answer -->\nOwner answer (relayed from Telegram, {how}):\n\n> {answer}"])
    print(json.dumps({"ok": p.returncode == 0, "issue": issue, "routedBy": how,
                      "answer": answer}))
    return 0 if p.returncode == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
