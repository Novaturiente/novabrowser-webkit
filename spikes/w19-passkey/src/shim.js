// w19 spike: WebAuthn API for WebKitGTK (built without ENABLE_WEB_AUTHN). Requests go to the
// UI process (`nbPasskey` reply handler), which runs them on a phone over caBLE/hybrid and
// checks rpId against the page's real origin itself. Main frame only.
(() => {
  if (window.PublicKeyCredential || !window.webkit?.messageHandlers?.nbPasskey) return;
  const post = (m) => window.webkit.messageHandlers.nbPasskey.postMessage(JSON.stringify(m));

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
    static getClientCapabilities() { return Promise.resolve({ hybridTransport: true, passkeyPlatformAuthenticator: false, userVerifyingPlatformAuthenticator: false, conditionalGet: false, conditionalCreate: false }); }
    static parseRequestOptionsFromJSON(o) {
      return { ...o, challenge: unb64u(o.challenge), allowCredentials: (o.allowCredentials || []).map((c) => ({ ...c, id: unb64u(c.id) })) };
    }
    static parseCreationOptionsFromJSON(o) {
      return { ...o, challenge: unb64u(o.challenge), user: { ...o.user, id: unb64u(o.user.id) }, excludeCredentials: (o.excludeCredentials || []).map((c) => ({ ...c, id: unb64u(c.id) })) };
    }
    getClientExtensionResults() { return this._ext || {}; }
    toJSON() { return this._json; }
  }
  const make = (proto, props) => {
    const o = Object.create(proto);
    for (const [k, v] of Object.entries(props)) Object.defineProperty(o, k, { value: v, enumerable: !k.startsWith("_") });
    return o;
  };
  const credential = (j, create) => {
    const r = j.response;
    const response = create
      ? make(AuthenticatorAttestationResponse.prototype, { clientDataJSON: unb64u(r.clientDataJSON), attestationObject: unb64u(r.attestationObject), _transports: r.transports })
      : make(AuthenticatorAssertionResponse.prototype, {
          clientDataJSON: unb64u(r.clientDataJSON), authenticatorData: unb64u(r.authenticatorData),
          signature: unb64u(r.signature), userHandle: r.userHandle ? unb64u(r.userHandle) : null,
        });
    return make(PublicKeyCredential.prototype, {
      id: j.id, rawId: unb64u(j.rawId), type: "public-key", authenticatorAttachment: "cross-platform", response,
      _ext: j.extensions || {}, _json: { id: j.id, rawId: j.rawId, type: "public-key", authenticatorAttachment: "cross-platform", response: r, clientExtensionResults: j.extensions || {} },
    });
  };

  const run = (op, options) => {
    const pk = options && options.publicKey;
    if (!pk) return op === "get" ? Promise.resolve(null) : Promise.reject(new DOMException("Only publicKey credentials are supported", "NotSupportedError"));
    // autofill passkeys (conditional UI) are not offered: such a request just waits
    if (options.mediation === "conditional") return new Promise((_, no) => options.signal?.addEventListener("abort", () => no(new DOMException("aborted", "AbortError"))));
    if (options.signal?.aborted) return Promise.reject(new DOMException("aborted", "AbortError"));
    const publicKey = enc(pk);
    if (op === "get" && !publicKey.rpId) publicKey.rpId = location.hostname;
    if (op === "create" && !publicKey.rp.id) publicKey.rp.id = location.hostname;
    return new Promise((ok, no) => {
      // shortcut: abort only rejects here; the phone request runs on until it times out
      options.signal?.addEventListener("abort", () => no(new DOMException("aborted", "AbortError")));
      post({ op, publicKey }).then((s) => {
        const r = JSON.parse(s);
        if (r.error) no(new DOMException(r.message || r.error, r.error));
        else ok(credential(r.credential, op === "create"));
      }, (e) => no(new DOMException(String(e), "NotAllowedError")));
    });
  };
  class CredentialsContainer {
    get(o) { return run("get", o); }
    create(o) { return run("create", o); }
    store() { return Promise.reject(new DOMException("not supported", "NotSupportedError")); }
    preventSilentAccess() { return Promise.resolve(); }
  }
  const container = new CredentialsContainer();
  Object.defineProperty(Navigator.prototype, "credentials", { get: () => container, configurable: true });
  Object.assign(window, { Credential, CredentialsContainer, PublicKeyCredential, AuthenticatorResponse, AuthenticatorAssertionResponse, AuthenticatorAttestationResponse });
})();
