#!/usr/bin/env bash
# Passkey shim checks (src/passkey.rs, ui/passkey*.js) against a local HTTPS server with a
# self-signed certificate for 127.0.0.1 and localhost (two origins: top page + iframe).
# The phone ceremony itself needs a person and a phone: see docs/manual-test.md.
# Needs: openssl, python3, a Wayland/X11 session.
# Usage: tests/passkey.sh [path/to/novabrowser-webkit]
set -u
. "$(dirname "$0")/lib.sh"
command -v openssl >/dev/null || { echo "skip: no openssl"; exit 0; }

srv=$tmp/www; mkdir -p $srv
port=$((20000 + RANDOM % 20000))
# top page on 127.0.0.1 (an IP: rpId check must refuse), iframe on localhost (cross-origin)
cat > $srv/top.html <<EOF
<title>wait</title>
<iframe src="https://localhost:$port/frame.html"></iframe>
<script>
const r = ['pkc=' + typeof PublicKeyCredential, 'creds=' + typeof navigator.credentials,
  'handler=' + typeof (window.webkit && webkit.messageHandlers && webkit.messageHandlers.novaPasskey)];
let frame = null;
const show = () => { if (frame && r.length === 5) document.title = 'R ' + r.join(' ') + ' | ' + frame; };
addEventListener('message', (e) => { frame = e.data; show(); });
const t = (name, pk) => navigator.credentials.get({ publicKey: pk }).then(() => name + '=ok', (e) => name + '=' + e.name);
Promise.all([t('ip', { challenge: new Uint8Array(16) }), t('evil', { challenge: new Uint8Array(16), rpId: 'evil.com' })])
  .then((x) => { r.push(...x); show(); });
</script>
EOF
cat > $srv/frame.html <<'EOF'
<title>frame</title>
<script>
let a = 'pkc=' + typeof PublicKeyCredential;
try { parent.document.dispatchEvent(new CustomEvent('novapasskey-req', { detail: '{"id":1,"op":"get","publicKey":{}}' })); a += ' reach=yes'; }
catch (e) { a += ' reach=no'; }
a += ' handler=' + typeof (window.webkit && webkit.messageHandlers && webkit.messageHandlers.novaPasskey);
parent.postMessage(a, '*');
</script>
EOF
openssl req -x509 -newkey rsa:2048 -nodes -days 1 -subj /CN=127.0.0.1 \
    -addext subjectAltName=IP:127.0.0.1,DNS:localhost -keyout $tmp/key.pem -out $tmp/cert.pem 2>/dev/null
(cd $srv && exec python3 -c "
import http.server, ssl
s = http.server.HTTPServer(('127.0.0.1', $port), http.server.SimpleHTTPRequestHandler)
c = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); c.load_cert_chain('$tmp/cert.pem', '$tmp/key.pem')
s.socket = c.wrap_socket(s.socket, server_side=True)
s.serve_forever()" 2>$tmp/http.log) &
http=$!
sleep 0.5

start "$(page start)"
wait_for '"title":"start"' 30 || fail "start"
# self-signed: allow the certificate once per host (TLS prompt, y)
for u in "https://localhost:$port/frame.html" "https://127.0.0.1:$port/top.html"; do
    "$bin" --cmd "open $u" >/dev/null
    wait_for '"prompt":"[^"]*(certificate|TLS|secure)' 10 || fail "tls prompt for $u: $(q)"
    sleep 1   # past the prompt's grace period
    "$bin" --key y >/dev/null
    sleep 1
done
wait_for '"title":"R ' 15 || fail "top page result: $(q)"
res=$(q | grep -o '"title":"R [^"]*"')
echo "     $res"
grep -q 'pkc=function creds=object' <<<"$res" && ok "main frame gets PublicKeyCredential + navigator.credentials" || fail "shim missing"
grep -q 'R [^|]*handler=undefined' <<<"$res" && ok "page world can't post to the passkey handler" || fail "handler visible to the page"
grep -q 'ip=SecurityError' <<<"$res" && ok "rpId on an IP origin refused" || fail "IP origin not refused"
grep -q 'evil=SecurityError' <<<"$res" && ok "foreign rpId refused" || fail "foreign rpId not refused"
grep -q '| pkc=undefined reach=no handler=undefined' <<<"$res" && ok "cross-origin iframe: no API, can't reach the bridge or handler" || fail "iframe isolation"
stop
finish
