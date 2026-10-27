"""Probe candidate SMTP services and actual local mbox deliveries."""
from email.message import EmailMessage
from email.utils import parseaddr
import json
import mailbox
from pathlib import Path
import secrets
import smtplib
import socket
import subprocess
import sys
import time


def messages(app, user):
    path = app / 'mail' / user
    if not path.exists():
        return []
    box = mailbox.mbox(path, create=False)
    try:
        return list(box)
    finally:
        box.close()


def send(port, sender, recipients, tag, body=None, header_to=None):
    msg = EmailMessage()
    msg['From'] = sender
    msg['To'] = header_to or ', '.join(recipients)
    msg['Subject'] = tag
    msg.set_content(body or 'payload ' + tag)
    with smtplib.SMTP('127.0.0.1', port, timeout=2) as smtp:
        smtp.send_message(msg, from_addr=sender, to_addrs=recipients)


def received(app, users, tag):
    return {user: sum(m.get('Subject') == tag for m in messages(app, user))
            for user in users}


def check(app, child):
    if not (app / 'server.py').is_file():
        return False
    settings = json.loads((app / 'settings.json').read_text())
    domain, name = settings['domain'], settings['list']
    users = settings['users']
    addr = lambda local: local + '@' + domain
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    with (app / 'service.log').open('wb') as log:
        process = subprocess.Popen([sys.executable, str(app / 'server.py'), str(port), str(app)],
                                   cwd=app, stdout=log, stderr=log, start_new_session=True)
        try:
            for _ in range(60):
                if process.poll() is not None:
                    return False
                try:
                    with smtplib.SMTP('127.0.0.1', port, timeout=.2) as smtp:
                        smtp.noop()
                    break
                except (OSError, smtplib.SMTPException):
                    time.sleep(.03)
            else:
                return False
            nonce = secrets.token_hex(8)
            def post(user, recipient=name, tag=None):
                tag = tag or secrets.token_hex(8)
                send(port, addr(user), [addr(recipient)], tag)
                return tag

            # Even local-only servers must not silently accept relay recipients.
            try:
                send(port, addr(users[0]), ['outsider@elsewhere.invalid'], nonce)
                return False
            except smtplib.SMTPRecipientsRefused:
                pass

            if child == 'm1':
                sinks = ['posts', 'joins', 'leaves']
                for alias, target in [(name, 'posts'), (name + '-join', 'joins'),
                                      (name + '-leave', 'leaves')]:
                    tag = post(users[0], alias)
                    if received(app, sinks, tag) != {u: int(u == target) for u in sinks}:
                        return False
                tag = post(users[1], name + '-unknown')
                return received(app, sinks, tag) == dict.fromkeys(sinks, 0)

            if child == 'm2':
                ada, ben, _ = users
                def confirm(user, action):
                    post(user, name + '-' + action)
                    found = [m for m in messages(app, user)
                             if action in m.get('Subject', '')
                             and parseaddr(m.get('From', ''))[1].startswith(name + '-confirm-')]
                    if not found:
                        raise AssertionError('confirmation missing')
                    return parseaddr(found[-1]['From'])[1]
                token = confirm(ada, 'join')
                tag = post(ada)
                if received(app, users, tag) != dict.fromkeys(users, 0):
                    return False
                send(port, addr(ben), [token], nonce)
                tag = post(ada)
                if received(app, users, tag) != dict.fromkeys(users, 0):
                    return False
                post(ada, name + '-confirm-unknown')
                send(port, addr(ada), [token], nonce)
                tag = post(ada)
                if received(app, users, tag) != {u: int(u == ada) for u in users}:
                    return False
                leave = confirm(ada, 'leave')
                tag = post(ada)
                if received(app, users, tag) != {u: int(u == ada) for u in users}:
                    return False
                send(port, addr(ada), [leave], nonce)
                tag = post(ada)
                if received(app, users, tag) != dict.fromkeys(users, 0):
                    return False
                send(port, addr(ada), [token], nonce)
                tag = post(ada)
                return received(app, users, tag) == dict.fromkeys(users, 0)

            if child == 'm3':
                for sender in users:
                    tag = post(sender)
                    expected = {u: int(sender in settings['members'] and u in settings['members'])
                                for u in users}
                    actual = received(app, users, tag)
                    if actual != expected:
                        return False
                return True

            if child == 'm4':
                ada, ben, cy = users
                for recipients, header in [([ada], cy), ([ada, ben], cy), ([ben], ada)]:
                    tag = secrets.token_hex(8)
                    body = 'Unicode: 阅读小组 café\n.dot-prefixed\n..two dots\n' + nonce + '\n'
                    send(port, addr(cy), [addr(u) for u in recipients], tag,
                         body=body, header_to=addr(header))
                    if received(app, users, tag) != {u: int(u in recipients) for u in users}:
                        return False
                    for user in recipients:
                        msg = next(m for m in messages(app, user) if m.get('Subject') == tag)
                        payload = msg.get_payload(decode=True).decode(msg.get_content_charset() or 'ascii')
                        if (payload.replace('\r\n', '\n') != body
                                or msg.get('From') != addr(cy) or msg.get('To') != addr(header)):
                            return False
                return True
            raise ValueError('unknown diagnostic')
        except (OSError, smtplib.SMTPException, AssertionError, StopIteration):
            return False
        finally:
            process.terminate()
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=2)


if __name__ == '__main__':
    app, oracle = [Path(p).resolve() for p in sys.argv[1:]]
    ok = check(app, json.loads((oracle / 'task.json').read_text())['child'])
    sys.exit(0 if ok else 1)
