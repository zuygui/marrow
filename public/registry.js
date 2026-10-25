// The package registry, read in the browser: nothing here is built into the site, so a new
// package shows up as soon as the registry's index.json changes. Everything that comes from
// the registry or from a package's repository is inserted as text (never as HTML).
(function () {
  "use strict";
  var SOURCES = [
    "https://raw.githubusercontent.com/zuygui/marrow/registry/",
    "https://cdn.jsdelivr.net/gh/zuygui/marrow@registry/"
  ];
  var CACHE_KEY = "marrow-registry-index";
  var BASE = "/marrow/";

  var root = document.getElementById("registry-app");
  if (!root) return;
  var mode = root.getAttribute("data-mode");

  function el(tag, attrs, children) {
    var e = document.createElement(tag);
    if (attrs) for (var k in attrs) {
      if (k === "class") e.className = attrs[k];
      else if (k === "text") e.textContent = attrs[k];
      else e.setAttribute(k, attrs[k]);
    }
    (children || []).forEach(function (c) { e.appendChild(typeof c === "string" ? document.createTextNode(c) : c); });
    return e;
  }

  function injectStyles() {
    var css =
      "#registry-app .reg-search{width:100%;padding:.6rem .8rem;font-size:1rem;border:1px solid var(--sl-color-gray-5);border-radius:.4rem;background:var(--sl-color-bg-nav);color:var(--sl-color-text)}" +
      "#registry-app .reg-meta{color:var(--sl-color-gray-3);font-size:.85rem;margin:.6rem 0}" +
      "#registry-app .reg-list{list-style:none;padding:0;margin:0}" +
      "#registry-app .reg-item{border:1px solid var(--sl-color-gray-5);border-radius:.4rem;padding:.7rem .9rem;margin:.6rem 0}" +
      "#registry-app .reg-item a.reg-name{font-weight:600;font-size:1.05rem}" +
      "#registry-app .reg-ver{color:var(--sl-color-gray-3);margin-left:.5rem;font-size:.85rem}" +
      "#registry-app .reg-desc{margin:.3rem 0 0}" +
      "#registry-app .reg-tags{margin-top:.4rem}" +
      "#registry-app .reg-tag{display:inline-block;margin:0 .3rem .2rem 0;padding:0 .5rem;border-radius:1rem;font-size:.75rem;background:var(--sl-color-gray-6);color:var(--sl-color-gray-2);border:0;cursor:pointer}" +
      "#registry-app .reg-error{color:var(--sl-color-red);margin:1rem 0}" +
      "#registry-app pre.reg-code{padding:.7rem .9rem;border-radius:.4rem;background:var(--sl-color-gray-6);overflow-x:auto}" +
      "#registry-app table.reg-table{width:100%}" +
      "#registry-app .reg-readme img{max-width:100%}";
    document.head.appendChild(el("style", { text: css }));
  }

  // The index: always asked of the network first (the browser revalidates it by ETag), the last
  // copy kept in localStorage is the fallback when offline.
  function fetchJson(i) {
    if (i >= SOURCES.length) return Promise.reject(new Error("The registry cannot be reached right now."));
    return fetch(SOURCES[i] + "index.json", { cache: "no-cache" })
      .then(function (r) { if (!r.ok) throw new Error("status " + r.status); return r.json(); })
      .catch(function () { return fetchJson(i + 1); });
  }
  function loadIndex() {
    return fetchJson(0).then(function (data) {
      try { localStorage.setItem(CACHE_KEY, JSON.stringify(data)); } catch (e) {}
      return { data: data, stale: false };
    }, function (err) {
      try {
        var saved = localStorage.getItem(CACHE_KEY);
        if (saved) return { data: JSON.parse(saved), stale: true };
      } catch (e) {}
      throw err;
    });
  }

  function fail(msg) { root.textContent = ""; root.appendChild(el("p", { class: "reg-error", text: msg })); }

  // ---- search ------------------------------------------------------------------------------
  function score(p, words) {
    var total = 0;
    var name = p.name.toLowerCase(), desc = (p.description || "").toLowerCase();
    var kws = (p.keywords || []).map(function (k) { return k.toLowerCase(); });
    for (var i = 0; i < words.length; i++) {
      var w = words[i], s = 0;
      if (name === w) s = 100;
      else if (name.indexOf(w) === 0) s = 60;
      else if (name.indexOf(w) >= 0) s = 40;
      else if (kws.indexOf(w) >= 0) s = 30;
      else if (kws.some(function (k) { return k.indexOf(w) >= 0; })) s = 15;
      else if (desc.indexOf(w) >= 0) s = 10;
      if (s === 0) return 0;
      total += s;
    }
    return total;
  }

  function packageLink(name) { return BASE + "registry/package/?name=" + encodeURIComponent(name); }

  function renderSearch(index, stale) {
    root.textContent = "";
    var box = el("input", { class: "reg-search", type: "search", placeholder: "Search packages by name, keyword or description", "aria-label": "Search packages" });
    var meta = el("p", { class: "reg-meta" });
    var list = el("ul", { class: "reg-list" });
    root.appendChild(box); root.appendChild(meta); root.appendChild(list);
    var all = index.packages.slice().sort(function (a, b) { return a.name < b.name ? -1 : 1; });

    function show() {
      var words = box.value.toLowerCase().split(/\s+/).filter(Boolean);
      var found;
      if (words.length === 0) found = all;
      else found = all.map(function (p) { return { p: p, s: score(p, words) }; })
        .filter(function (x) { return x.s > 0; })
        .sort(function (a, b) { return b.s - a.s || (a.p.name < b.p.name ? -1 : 1); })
        .map(function (x) { return x.p; });
      meta.textContent = found.length + " package" + (found.length === 1 ? "" : "s") + (stale ? " (offline copy of the index)" : "");
      list.textContent = "";
      found.slice(0, 100).forEach(function (p) {
        var tags = el("div", { class: "reg-tags" });
        (p.keywords || []).forEach(function (k) {
          var t = el("button", { class: "reg-tag", type: "button", text: k });
          t.addEventListener("click", function () { box.value = k; show(); });
          tags.appendChild(t);
        });
        list.appendChild(el("li", { class: "reg-item" }, [
          el("a", { class: "reg-name", href: packageLink(p.name), text: p.name }),
          el("span", { class: "reg-ver", text: "v" + p.latest }),
          el("p", { class: "reg-desc", text: p.description || "" }),
          tags
        ]));
      });
      if (found.length === 0) list.appendChild(el("li", { class: "reg-meta", text: "Nothing matches. Publish your own with `marrow publish`." }));
    }
    box.addEventListener("input", show);
    var q = new URLSearchParams(location.search).get("q");
    if (q) box.value = q;
    show();
  }

  // ---- a package page -------------------------------------------------------------------------
  function githubRepo(url) {
    var m = /^https:\/\/github\.com\/([^\/]+)\/([^\/#?]+?)(?:\.git)?\/?$/.exec(url || "");
    return m ? { owner: m[1], name: m[2] } : null;
  }

  // A small Markdown renderer that builds DOM nodes: headings, paragraphs, lists, code, emphasis,
  // inline code and links to http(s) addresses. Anything else stays plain text.
  function inline(parent, text) {
    var re = /(`[^`]+`)|(\*\*[^*]+\*\*)|(\[[^\]]+\]\([^)\s]+\))/g, last = 0, m;
    while ((m = re.exec(text))) {
      if (m.index > last) parent.appendChild(document.createTextNode(text.slice(last, m.index)));
      var t = m[0];
      if (m[1]) parent.appendChild(el("code", { text: t.slice(1, -1) }));
      else if (m[2]) parent.appendChild(el("strong", { text: t.slice(2, -2) }));
      else {
        var parts = /^\[([^\]]+)\]\(([^)\s]+)\)$/.exec(t);
        if (/^https?:\/\//i.test(parts[2])) parent.appendChild(el("a", { href: parts[2], rel: "noopener noreferrer nofollow", text: parts[1] }));
        else parent.appendChild(document.createTextNode(parts[1]));
      }
      last = m.index + t.length;
    }
    if (last < text.length) parent.appendChild(document.createTextNode(text.slice(last)));
  }
  function markdown(src) {
    var out = el("div", { class: "reg-readme" });
    var lines = src.replace(/\r/g, "").split("\n"), i = 0;
    while (i < lines.length) {
      var line = lines[i], m;
      if (/^```/.test(line)) {
        var code = []; i++;
        while (i < lines.length && !/^```/.test(lines[i])) code.push(lines[i++]);
        i++;
        out.appendChild(el("pre", { class: "reg-code" }, [el("code", { text: code.join("\n") })]));
      } else if ((m = /^(#{1,4})\s+(.*)$/.exec(line))) {
        var h = el("h" + Math.min(m[1].length + 2, 6)); inline(h, m[2]); out.appendChild(h); i++;
      } else if (/^\s*[-*]\s+/.test(line)) {
        var ul = el("ul");
        while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) { var li = el("li"); inline(li, lines[i].replace(/^\s*[-*]\s+/, "")); ul.appendChild(li); i++; }
        out.appendChild(ul);
      } else if (line.trim() === "") { i++; }
      else {
        var para = [];
        while (i < lines.length && lines[i].trim() !== "" && !/^(```|#{1,4}\s|\s*[-*]\s)/.test(lines[i])) para.push(lines[i++]);
        var p = el("p"); inline(p, para.join(" ")); out.appendChild(p);
      }
    }
    return out;
  }

  function renderPackage(index) {
    var name = new URLSearchParams(location.search).get("name");
    if (!name) { fail("No package was asked for. Go back to the registry to pick one."); return; }
    var p = index.packages.filter(function (x) { return x.name === name; })[0];
    if (!p) { fail("There is no package called \"" + name + "\" in the registry."); return; }
    document.title = p.name + " | Marrow Registry";
    root.textContent = "";
    root.appendChild(el("p", { class: "reg-meta" }, [el("a", { href: BASE + "registry/", text: "← All packages" })]));
    root.appendChild(el("h2", { text: p.name + " " }, [el("span", { class: "reg-ver", text: "v" + p.latest })]));
    if (p.description) root.appendChild(el("p", { text: p.description }));
    var tags = el("div", { class: "reg-tags" });
    (p.keywords || []).forEach(function (k) { tags.appendChild(el("a", { class: "reg-tag", href: BASE + "registry/?q=" + encodeURIComponent(k), text: k })); });
    root.appendChild(tags);
    root.appendChild(el("h3", { text: "Install" }));
    root.appendChild(el("pre", { class: "reg-code" }, [el("code", { text: "marrow add " + p.name })]));
    root.appendChild(el("p", { text: "Then, in your program:" }));
    root.appendChild(el("pre", { class: "reg-code" }, [el("code", { text: '@import("' + p.name + '", ' + p.name.replace(/-/g, "_") + ')' })]));
    if (/^https?:\/\//.test(p.repo || "")) root.appendChild(el("p", {}, ["Source: ", el("a", { href: p.repo, rel: "noopener noreferrer", text: p.repo })]));
    root.appendChild(el("h3", { text: "Versions" }));
    var table = el("table", { class: "reg-table" }, [el("thead", {}, [el("tr", {}, [el("th", { text: "Version" }), el("th", { text: "Revision" })])])]);
    var body = el("tbody");
    p.versions.forEach(function (v) { body.appendChild(el("tr", {}, [el("td", { text: v.version }), el("td", {}, [el("code", { text: String(v.rev).slice(0, 10) })])])); });
    table.appendChild(body); root.appendChild(table);

    var gh = githubRepo(p.repo);
    if (gh) {
      var holder = el("div"); root.appendChild(el("h3", { text: "README" })); root.appendChild(holder);
      var rev = p.versions[0].rev;
      fetch("https://raw.githubusercontent.com/" + gh.owner + "/" + gh.name + "/" + rev + "/README.md")
        .then(function (r) { if (!r.ok) throw new Error(); return r.text(); })
        .then(function (t) { holder.appendChild(markdown(t.slice(0, 200000))); })
        .catch(function () { holder.appendChild(el("p", { class: "reg-meta", text: "No README could be loaded." })); });
    }
  }

  injectStyles();
  root.appendChild(el("p", { class: "reg-meta", text: "Loading the registry…" }));
  loadIndex().then(function (r) {
    if (mode === "package") renderPackage(r.data); else renderSearch(r.data, r.stale);
  }).catch(function (e) { fail(e.message || "The registry cannot be reached right now."); });
})();
