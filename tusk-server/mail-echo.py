import os.path
import quopri
import ssl
from encodings.utf_8 import decode

from aiosmtpd.controller import Controller
from aiosmtpd.smtp import LoginPassword, AuthResult


class EchoHandler:
    async def handle_RCPT(self, server, session, envelope, address, rcpt_options):
        if not address.endswith('@example.com'):
            return '550 not relaying to that domain'
        envelope.rcpt_tos.append(address)
        return '250 OK'

    async def handle_DATA(self, server, session, envelope):
        print('----------------------------------------------------------------')
        print('Message from %s' % envelope.mail_from)
        print('Message for %s' % envelope.rcpt_tos)
        # print('Message data:\n')
        decoded = quopri.decodestring(envelope.content.decode('utf8', errors='replace')).decode('utf-8')
        decoded = "\n".join(decoded.splitlines())
        with open('%s/test_srv/mail/%s.mail' % (os.path.dirname(__file__), envelope.rcpt_tos[0]), 'w') as file:
            file.write(decoded)
        print('Message can be found at %s/test_srv/mail/%s.mail' % (os.path.dirname(__file__), envelope.rcpt_tos[0]))
        # for ln in decoded.splitlines():
        #     print(f'> {ln}'.strip())
        # print()
        # print('End of message')
        return '250 Message accepted for delivery'


def authenticator(server, session, envelope, mechanism, auth_data):
    assert isinstance(auth_data, LoginPassword)

    if auth_data.login == b"smtp_admin" and auth_data.password == b"smtp_password":
        return AuthResult(success=True)
    else:
        return AuthResult(success=False, handled=False)


print('Setting up server...')
context = ssl.create_default_context(ssl.Purpose.CLIENT_AUTH)
context.load_cert_chain('cert.pem', 'key.pem')
controller = Controller(
    EchoHandler(),
    hostname='localhost',
    port=2525,
    authenticator=authenticator,
    auth_required=True,
    require_starttls=True,
    tls_context=context
)
print('Starting server...')
controller.start()
input("Server started. Press Return to quit.\n")
controller.stop()
