import datetime, json, ssl, subprocess, sys
cert = ssl._ssl._test_decode_cert(sys.argv[1])
date = lambda x: datetime.datetime.strptime(x, '%b %d %H:%M:%S %Y %Z')
days = (date(cert['notAfter'])-date(cert['notBefore'])).total_seconds()/86400
ext = subprocess.run(['openssl','x509','-in',sys.argv[1],'-noout','-ext','basicConstraints'], check=True, capture_output=True, text=True).stdout
sans = sorted(value for kind,value in cert.get('subjectAltName', ()) if kind == 'DNS')
valid = sans == ['api.sandbox.test','sandbox.test'] and 'CA:FALSE' in ext
print(json.dumps({'valid': valid}))
