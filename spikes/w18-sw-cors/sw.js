// Pass-through worker, like Angular's ngsw for requests it does not cache.
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));
// ?wait in the page URL: await cache storage first, as ngsw does before its fetch.
self.addEventListener("fetch", (e) =>
  e.respondWith(
    (async () => {
      if (new URL(e.request.url).searchParams.has("wait")) await caches.open("w18");
      return fetch(e.request);
    })(),
  ),
);
