//! The editorial CSS, copied verbatim from the Python template.

pub const CSS: &str = r##"

* { box-sizing: border-box; }
html, body {
  margin: 0; padding: 0; width: 100%; height: 100%; overflow: hidden;
}
body {
  --body-size: clamp(15px, calc(21px * var(--s)), 21px);
  --lead-size: clamp(17px, calc(26px * var(--s)), 26px);
  --desc-size: clamp(13px, calc(16px * var(--s)), 16px);
  --small-size: clamp(9px, calc(11px * var(--s)), 11px);
  --qr-size: clamp(64px, calc(104px * var(--s)), 104px);
  --pad-top: clamp(22px, calc(62px * var(--s)), 62px);
  --pad-right: clamp(24px, calc(70px * var(--s)), 70px);
  --pad-bottom: clamp(18px, calc(50px * var(--s)), 50px);
  --pad-left: clamp(28px, calc(76px * var(--s)), 76px);
  --gap: clamp(12px, calc(24px * var(--s)), 24px);
  background: var(--paper);
  color: var(--ink);
  font-family: "Source Serif 4", Georgia, "Times New Roman", serif;
  -webkit-font-smoothing: antialiased;
}
.canvas {
  width: 100vw; height: 100vh;
  display: grid; grid-template-columns: 58% 42%;
  overflow: hidden;
}
.panel {
  position: relative; overflow: hidden;
  display: flex; flex-direction: column;
  padding: var(--pad-top) var(--pad-right) var(--pad-bottom) var(--pad-left);
  background: var(--paper);
}
.panel::before {
  content: ""; position: absolute; inset: 0;
  background-image: radial-gradient(var(--texture) 0.7px, transparent 0.8px);
  background-size: 5px 5px; opacity: 0.55; pointer-events: none;
}
.panel::after {
  content: ""; position: absolute; left: 0; top: 0; bottom: 0;
  width: clamp(6px, calc(10px * var(--s)), 10px);
  background: var(--accent);
}
.watermark {
  position: absolute; right: calc(-46px * var(--s)); bottom: calc(-150px * var(--s));
  font-family: "Source Serif 4", Georgia, serif;
  font-size: clamp(280px, calc(520px * var(--s)), 520px);
  line-height: 1; color: var(--ink); opacity: 0.035;
  pointer-events: none; user-select: none;
}
.masthead {
  position: relative; z-index: 1;
  display: flex; align-items: flex-start; justify-content: space-between; gap: var(--gap);
}
.brand { display: flex; align-items: center; gap: clamp(8px, calc(14px * var(--s)), 14px); }
.mark {
  width: clamp(32px, calc(44px * var(--s)), 44px);
  height: clamp(32px, calc(44px * var(--s)), 44px);
  display: grid; place-items: center;
  background: var(--ink); color: var(--paper);
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: clamp(20px, calc(28px * var(--s)), 28px);
  font-weight: 700; line-height: 1;
}
.brand-name {
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: clamp(12px, calc(15px * var(--s)), 15px);
  font-weight: 700; letter-spacing: 0.24em;
}
.meta {
  display: flex; flex-direction: column; align-items: flex-end;
  text-align: right;
}
.kicker {
  display: block; color: var(--accent);
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: clamp(10px, calc(13px * var(--s)), 13px);
  font-weight: 700; letter-spacing: 0.18em;
}
.date {
  display: block; margin-top: clamp(4px, calc(7px * var(--s)), 7px);
  color: var(--muted);
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: clamp(9px, calc(12px * var(--s)), 12px);
  letter-spacing: 0.14em;
}
.skip {
  display: inline-flex; align-items: center; gap: 0.35em;
  margin-top: clamp(7px, calc(12px * var(--s)), 12px);
  padding: clamp(4px, calc(6px * var(--s)), 6px) clamp(7px, calc(11px * var(--s)), 11px);
  border: 1px solid var(--rule); border-radius: 999px;
  color: var(--ink); background: transparent;
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: clamp(8px, calc(10px * var(--s)), 10px);
  font-weight: 600; letter-spacing: 0.1em; text-transform: uppercase;
  white-space: nowrap;
}
.skip .arrow { color: var(--accent); font-size: 1.15em; }
.rule {
  position: relative; z-index: 1; height: 1px; background: var(--rule);
  margin: clamp(12px, calc(26px * var(--s)), 26px) 0 0;
}
.content {
  position: relative; z-index: 1; flex: 1; min-height: 0;
  padding-top: clamp(14px, calc(34px * var(--s)), 34px);
  display: flex; flex-direction: column; justify-content: center;
}
.title {
  font-family: "Source Serif 4", Georgia, "Times New Roman", serif;
  font-weight: 700; line-height: 0.98; letter-spacing: -0.035em;
  margin: 0; max-width: 980px; text-wrap: balance;
  font-size: var(--title-size);
}
.description {
  margin: clamp(10px, calc(24px * var(--s)), 24px) 0 0;
  max-width: 880px; color: var(--muted);
  font-family: "Source Serif 4", Georgia, serif;
  font-size: var(--desc-size); line-height: 1.4;
}
.lead {
  margin: clamp(12px, calc(28px * var(--s)), 28px) 0 0;
  max-width: 930px; color: var(--body);
  font-family: "Source Serif 4", Georgia, serif;
  font-size: var(--lead-size); line-height: 1.42;
}
.lead::first-letter {
  float: left; font-size: 3.1em; line-height: 0.78;
  padding: 0.06em 0.09em 0 0;
  color: var(--accent); font-weight: 700;
}
.lead.no-dropcap::first-letter {
  float: none; font-size: inherit; line-height: inherit;
  padding: 0; color: inherit; font-weight: inherit;
}
.extract {
  margin: clamp(9px, calc(16px * var(--s)), 16px) 0 0;
  max-width: 930px; color: var(--body);
  font-family: "Source Serif 4", Georgia, serif;
  font-size: var(--body-size); line-height: 1.62;
}
.footer {
  position: relative; z-index: 1;
  display: flex; align-items: flex-end; justify-content: space-between; gap: var(--gap);
  padding-top: clamp(10px, calc(24px * var(--s)), 24px);
  border-top: 1px solid var(--rule);
}
.source { min-width: 0; flex: 1; }
.url {
  display: flex; align-items: baseline; gap: 4px; max-width: 100%;
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: clamp(12px, calc(15px * var(--s)), 15px);
  font-weight: 600; letter-spacing: 0.01em;
  color: var(--ink);
  text-decoration: underline;
  text-decoration-color: var(--accent);
  text-decoration-thickness: 1px;
  text-underline-offset: 5px;
}
.url-text { min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.url .arrow { flex-shrink: 0; color: var(--accent); }
.license {
  margin-top: clamp(4px, calc(8px * var(--s)), 8px);
  color: var(--muted);
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: var(--small-size);
  letter-spacing: 0.12em; text-transform: uppercase;
  text-wrap: balance;
}
.qr {
  display: flex; align-items: center;
  gap: clamp(8px, calc(14px * var(--s)), 14px);
  flex-shrink: 0;
}
.qr img {
  width: var(--qr-size); height: var(--qr-size); display: block;
  padding: clamp(5px, calc(7px * var(--s)), 7px);
  background: var(--qr-bg); border: 1px solid var(--rule);
}
.qr span {
  width: clamp(70px, calc(100px * var(--s)), 100px);
  color: var(--muted);
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: clamp(9px, calc(10px * var(--s)), 10px);
  line-height: 1.45; letter-spacing: 0.12em; text-transform: uppercase;
}
.hero {
  position: relative; overflow: hidden;
  background: linear-gradient(135deg, var(--hero-a), var(--hero-b));
}
.hero-img {
  position: absolute; inset: 0; width: 100%; height: 100%;
  object-fit: cover; filter: saturate(0.92) contrast(1.02);
}
.hero::after {
  content: ""; position: absolute; inset: 0;
  background:
    linear-gradient(180deg, rgba(0, 0, 0, 0.42) 0%, rgba(0, 0, 0, 0) 18%),
    linear-gradient(0deg, rgba(0, 0, 0, 0.48) 0%, rgba(0, 0, 0, 0) 42%),
    linear-gradient(90deg, var(--paper) 0%, rgba(0, 0, 0, 0) 9%);
}
.hero-badge {
  position: absolute; z-index: 2;
  top: clamp(16px, calc(34px * var(--s)), 34px);
  right: clamp(18px, calc(38px * var(--s)), 38px);
  color: rgba(255, 255, 255, 0.86);
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: clamp(9px, calc(11px * var(--s)), 11px);
  font-weight: 700; letter-spacing: 0.2em; text-transform: uppercase;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.35);
}
.hero-letter {
  position: absolute; inset: 0; display: grid; place-items: center;
  font-family: "Source Serif 4", Georgia, serif;
  font-size: clamp(220px, calc(420px * var(--s)), 420px);
  line-height: 1; color: rgba(255, 255, 255, 0.13);
}
.hero-caption {
  position: absolute; z-index: 2; left: 0; right: 0;
  bottom: clamp(16px, calc(34px * var(--s)), 34px);
  padding: 0 clamp(18px, calc(38px * var(--s)), 38px);
  color: rgba(255, 255, 255, 0.78);
  font-family: "Montserrat", "Segoe UI", sans-serif;
  font-size: clamp(8px, calc(10px * var(--s)), 10px);
  letter-spacing: 0.18em; text-transform: uppercase;
  text-shadow: 0 1px 8px rgba(0, 0, 0, 0.45);
}
"##;
