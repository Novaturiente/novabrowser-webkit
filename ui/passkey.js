// Passkeys (WebAuthn) for WebKitGTK, which ships without them. Page world, top frame,
// document start. Gives the page navigator.credentials / PublicKeyCredential; requests go
// as `novapasskey-req` events to passkey-bridge.js in our isolated world, which adds the
// frame's real origin and asks the browser (src/passkey.rs). Cross-origin iframes can't
// reach the top document, so only the main page can ask.
(() => {
  if (window !== window.top || window.PublicKeyCredential || location.protocol !== "https:") return;
  const doc = document;
  const dispatch = EventTarget.prototype.dispatchEvent;
  const Custom = CustomEvent;
  const parse = JSON.parse, str = JSON.stringify;
  let seq = 0;
  const waiting = new Map();
  doc.addEventListener("novapasskey-res", (e) => {
    if (typeof e.detail !== "string") return;
    const r = parse(e.detail);
    const ok = waiting.get(r.id);
    if (ok) { waiting.delete(r.id); ok(r.reply); }
  });
  const send = (m) => new Promise((ok) => {
    const id = ++seq;
    waiting.set(id, ok);
    dispatch.call(doc, new Custom("novapasskey-req", { detail: str({ ...m, id }) }));
    m.id = id;
  });

  const bytes = (v) => v instanceof ArrayBuffer ? new Uint8Array(v) : new Uint8Array(v.buffer, v.byteOffset, v.byteLength);
  const b64u = (v) => {
    let s = "";
    for (const x of bytes(v)) s += String.fromCharCode(x);
    return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  };
  const unb64u = (s) => {
    s = s.replace(/-/g, "+").replace(/_/g, "/");
    const bin = atob(s + "===".slice((s.length + 3) % 4));
    const u = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
    return u.buffer;
  };
  // BufferSource anywhere in the options -> base64url (the JSON form webauthn-rs reads)
  const enc = (v) =>
    v instanceof ArrayBuffer || ArrayBuffer.isView(v) ? b64u(v)
    : Array.isArray(v) ? v.map(enc)
    : v && typeof v === "object" ? Object.fromEntries(Object.entries(v).map(([k, x]) => [k, enc(x)]))
    : v;

  class Credential {}
  class AuthenticatorResponse {}
  class AuthenticatorAssertionResponse extends AuthenticatorResponse {}
  class AuthenticatorAttestationResponse extends AuthenticatorResponse {
    getTransports() { return this._transports || ["hybrid"]; }
  }
  class PublicKeyCredential extends Credential {
    static isUserVerifyingPlatformAuthenticatorAvailable() { return Promise.resolve(false); }
    static isConditionalMediationAvailable() { return Promise.resolve(false); }
    static getClientCapabilities() {
      return Promise.resolve({ hybridTransport: true, passkeyPlatformAuthenticator: false, userVerifyingPlatformAuthenticator: false, conditionalGet: false, conditionalCreate: false });
    }
    static parseRequestOptionsFromJSON(o) {
      return { ...o, challenge: unb64u(o.challenge), allowCredentials: (o.allowCredentials || []).map((c) => ({ ...c, id: unb64u(c.id) })) };
    }
    static parseCreationOptionsFromJSON(o) {
      return { ...o, challenge: unb64u(o.challenge), user: { ...o.user, id: unb64u(o.user.id) }, excludeCredentials: (o.excludeCredentials || []).map((c) => ({ ...c, id: unb64u(c.id) })) };
    }
    getClientExtensionResults() { return this._ext; }
    toJSON() { return this._json; }
  }
  const make = (proto, props) => {
    const o = Object.create(proto);
    for (const [k, v] of Object.entries(props)) Object.defineProperty(o, k, { value: v, enumerable: !k.startsWith("_") });
    return o;
  };
  const credential = (j, create) => {
    const r = j.response;
    // webauthn-rs writes every extension, unused ones as null; browsers leave them out
    const ext = Object.fromEntries(Object.entries(j.extensions || {}).filter(([, v]) => v != null));
    const response = create
      ? make(AuthenticatorAttestationResponse.prototype, { clientDataJSON: unb64u(r.clientDataJSON), attestationObject: unb64u(r.attestationObject), _transports: r.transports })
      : make(AuthenticatorAssertionResponse.prototype, {
          clientDataJSON: unb64u(r.clientDataJSON), authenticatorData: unb64u(r.authenticatorData),
          signature: unb64u(r.signature), userHandle: r.userHandle ? unb64u(r.userHandle) : null,
        });
    return make(PublicKeyCredential.prototype, {
      id: j.id, rawId: unb64u(j.rawId), type: "public-key", authenticatorAttachment: "cross-platform", response,
      _ext: ext, _json: { id: j.id, rawId: j.rawId, type: "public-key", authenticatorAttachment: "cross-platform", response: r, clientExtensionResults: ext },
    });
  };

  const abortError = () => new DOMException("The operation was aborted.", "AbortError");
  const run = (op, options) => {
    const pk = options && options.publicKey;
    if (!pk) return op === "get" ? Promise.resolve(null) : Promise.reject(new DOMException("Only publicKey credentials are supported", "NotSupportedError"));
    const signal = options.signal;
    // autofill passkeys (conditional UI) are not offered: such a request just waits
    if (options.mediation === "conditional") return new Promise((_, no) => signal?.addEventListener("abort", () => no(abortError())));
    if (signal?.aborted) return Promise.reject(abortError());
    let publicKey;
    try { publicKey = enc(pk); } catch (e) { return Promise.reject(new TypeError(String(e))); }
    return new Promise((ok, no) => {
      const m = { op, publicKey };
      // abort: reject now and close the browser's QR window (the phone side runs on)
      signal?.addEventListener("abort", () => { no(abortError()); send({ op: "cancel", target: m.id }); });
      send(m).then((s) => {
        const r = parse(s);
        if (r.error) no(new DOMException(r.message || r.error, r.error));
        else ok(credential(r.credential, op === "create"));
      });
    });
  };
  class CredentialsContainer {
    get(o) { return run("get", o); }
    create(o) { return run("create", o); }
    store() { return Promise.reject(new DOMException("Not supported", "NotSupportedError")); }
    preventSilentAccess() { return Promise.resolve(); }
  }
  const container = new CredentialsContainer();
  Object.defineProperty(Navigator.prototype, "credentials", { get: () => container, configurable: true, enumerable: true });
  for (const [k, v] of Object.entries({ Credential, CredentialsContainer, PublicKeyCredential, AuthenticatorResponse, AuthenticatorAssertionResponse, AuthenticatorAttestationResponse }))
    Object.defineProperty(window, k, { value: v, configurable: true, writable: true });
})();
