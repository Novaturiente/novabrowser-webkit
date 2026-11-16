// Passkey bridge, our isolated world, top frame only: passes passkey.js's requests to the
// browser with this frame's origin. The page can't change our world's `self`/`location`,
// so the origin can't be faked; src/passkey.rs also checks it against the view's URI.
(() => {
  if (window !== window.top || location.protocol !== "https:") return;
  // self.origin is the document's security origin: "null" when sandboxed (CSP sandbox),
  // where location.origin would still name the site
  const origin = () => (self.origin === location.origin ? self.origin : null);
  document.addEventListener("novapasskey-req", (e) => {
    if (typeof e.detail !== "string") return;
    let m;
    try { m = JSON.parse(e.detail); } catch (_) { return; }
    if (!Number.isInteger(m.id)) return;
    const msg = JSON.stringify({ id: m.id, op: m.op, target: m.target, publicKey: m.publicKey, origin: origin() });
    webkit.messageHandlers.novaPasskey.postMessage(msg).then((reply) => {
      document.dispatchEvent(new CustomEvent("novapasskey-res", { detail: JSON.stringify({ id: m.id, reply }) }));
    });
  });
})();
