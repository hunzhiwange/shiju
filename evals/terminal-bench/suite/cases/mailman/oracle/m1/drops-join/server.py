"""Local synthetic SMTP list receiver; no external delivery."""
import email
from email.message import EmailMessage
import json
import mailbox
from pathlib import Path
import secrets
import socketserver
import sys

BUG = 'drops-join'
root = Path(sys.argv[2])
settings = json.loads((root / 'settings.json').read_text())
mode = settings['mode']
domain = settings['domain']
name = settings['list']
users = set(settings['users'])
members = set(settings.get('members', []))
pending = {}

def deliver(user, message):
    folder = root / 'mail'
    folder.mkdir(exist_ok=True)
    box = mailbox.mbox(folder / user)
    try:
        if BUG == 'drop-body':
            message = email.message_from_string('Subject: ' + message['Subject'] + '\n\n')
        box.add(message)
        if BUG == 'duplicate':
            box.add(message)
        box.flush()
    finally:
        box.close()

def confirmation(user, action):
    token = secrets.token_hex(12)
    pending[token] = (user, action)
    reply = EmailMessage()
    reply['From'] = f'{name}-confirm-{token}@{domain}'
    reply['To'] = f'{user}@{domain}'
    reply['Subject'] = action + ' confirmation'
    reply.set_content('Reply to the sender to confirm ' + action)
    deliver(user, reply)

def dispatch(sender, recipients, message):
    user = sender.split('@')[0]
    for recipient in recipients:
        local, dest = recipient.rsplit('@', 1)
        if dest != domain:
            continue
        if mode == 'm1':
            routes = json.loads((root / 'routes.json').read_text())
            if BUG == 'drops-join' and local.endswith('-join'):
                continue
            if BUG == 'swaps-leave' and local.endswith('-leave'):
                local = name + '-join'
            target = routes.get(local)
            if target:
                deliver(target, message)
        elif mode == 'm2':
            if local in (name + '-join', name + '-leave'):
                action = 'join' if local.endswith('-join') else 'leave'
                if BUG == 'eager':
                    members.add(user) if action == 'join' else members.discard(user)
                confirmation(user, action)
            elif local.startswith(name + '-confirm-'):
                token = local.removeprefix(name + '-confirm-')
                record = pending.get(token)
                if record and (record[0] == user or BUG == 'wrong-sender'):
                    target, action = record
                    members.add(target) if action == 'join' else members.discard(target)
                    if BUG != 'replay':
                        del pending[token]
            elif local == name:
                for member in sorted(members):
                    deliver(member, message)
        elif mode == 'm3' and local == name:
            if user not in members and BUG != 'outsider-post':
                continue
            targets = users if BUG == 'leak' else members
            for member in sorted(targets):
                if BUG != 'omit-sender' or member != user:
                    deliver(member, message)
        elif mode == 'm4' and local in users:
            deliver(local, message)

class SMTP(socketserver.StreamRequestHandler):
    def handle(self):
        self.wfile.write(b'220 sandbox ESMTP\r\n')
        sender = ''
        recipients = []
        while line := self.rfile.readline(65536):
            command = line.decode().strip()
            verb = command.split(' ', 1)[0].upper()
            if verb in ('EHLO', 'HELO'):
                self.wfile.write(b'250 sandbox\r\n')
            elif verb == 'MAIL':
                sender = command.split('<', 1)[1].split('>', 1)[0]
                recipients = []
                self.wfile.write(b'250 ok\r\n')
            elif verb == 'RCPT':
                address = command.split('<', 1)[1].split('>', 1)[0]
                if address.count('@') != 1 or address.rsplit('@', 1)[1] != domain:
                    self.wfile.write(b'550 local domain only\r\n')
                else:
                    recipients.append(address)
                    self.wfile.write(b'250 ok\r\n')
            elif verb == 'DATA':
                self.wfile.write(b'354 end with dot\r\n')
                data = []
                while (part := self.rfile.readline(65536)) != b'.\r\n':
                    if not part:
                        return
                    data.append(part[1:] if part.startswith(b'..') else part)
                dispatch(sender, recipients, email.message_from_bytes(b''.join(data)))
                self.wfile.write(b'250 delivered\r\n')
            elif verb == 'RSET':
                recipients = []
                self.wfile.write(b'250 reset\r\n')
            elif verb == 'QUIT':
                self.wfile.write(b'221 bye\r\n')
                return
            else:
                self.wfile.write(b'250 ok\r\n')

with socketserver.TCPServer(('127.0.0.1', int(sys.argv[1])), SMTP) as service:
    service.serve_forever()
