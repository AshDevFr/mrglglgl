// Light or dark, and the paper: the reader's own choice if they made one here, else their
// system's light or dark on the paper the site was published with. Printing is always light.
(function () {
  var root = document.documentElement, key = "mrglglgl.site.";
  var attr = { scheme: "data-scheme", paper: "data-paper" };
  function saved(name) { try { return localStorage.getItem(key + name); } catch (e) { return null; } }
  function show(name, value, keep) {
    root.setAttribute(attr[name], value);
    if (keep) { try { localStorage.setItem(key + name, value); } catch (e) {} }
  }
  var dark = window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  show("scheme", saved("scheme") || (dark ? "dark" : "light"));
  show("paper", saved("paper") || root.getAttribute(attr.paper) || "tide");
  document.addEventListener("click", function (event) {
    var button = event.target.closest && event.target.closest("[data-toggle]");
    if (!button) return;
    var name = button.getAttribute("data-toggle");
    var now = root.getAttribute(attr[name]);
    show(name, name === "scheme" ? (now === "dark" ? "light" : "dark")
      : (now === "parchment" ? "tide" : "parchment"), true);
  });
  var before;
  window.addEventListener("beforeprint", function () {
    before = root.getAttribute(attr.scheme);
    root.setAttribute(attr.scheme, "light");
  });
  window.addEventListener("afterprint", function () {
    if (before) root.setAttribute(attr.scheme, before);
  });
})();
