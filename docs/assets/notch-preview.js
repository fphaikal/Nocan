(() => {
  "use strict";

  const sections = [
    "music",
    "focus",
    "water",
    "files",
    "tasks",
    "openmaus",
    "usage",
    "stocks",
  ];
  const names = [
    "Music",
    "Focus",
    "Water",
    "Files",
    "Tasks",
    "OpenMaus",
    "AI Usage",
    "Stocks",
  ];
  const paths = {
    target:
      '<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="4"/><path d="m12 12 9-9m-5 0h5v5"/>',
    coffee:
      '<path d="M4 8h12v8a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4Zm12 0h2a3 3 0 0 1 0 6h-2M7 2v3m5-3v3M2 22h18"/>',
    moon: '<path d="M20 15A9 9 0 0 1 9 3a9 9 0 1 0 11 12Z"/>',
    reset: '<path d="M4 8a8 8 0 1 1-1 7M4 3v5h5"/>',
    next: '<path fill="currentColor" stroke="none" d="m5 5 10 7L5 19Zm11 0h3v14h-3Z"/>',
    previous:
      '<path fill="currentColor" stroke="none" d="m19 5-10 7 10 7ZM5 5h3v14H5Z"/>',
    power:
      '<rect x="2" y="3" width="19" height="12" rx="2"/><path d="M5 6h13v6H5m16-5h2v4m-11 4v3m-4 0h8m-5 2 2 0-1 3"/>',
    plus: '<path d="M12 4v16M4 12h16"/>',
    edit: '<path d="m4 15 12-12 5 5L9 20l-6 1Zm10-10 5 5"/>',
    trash: '<path d="M3 6h18M9 6V3h6v3M6 6l1 15h10l1-15M10 9v9m4-9v9"/>',
    lyrics:
      '<rect x="3" y="3" width="18" height="17" rx="3"/><path d="M7 8h10M7 12h7m-7 4h10"/>',
    sliders:
      '<path d="M4 6h16M4 12h16M4 18h16"/><circle cx="8" cy="6" r="2"/><circle cx="16" cy="12" r="2"/><circle cx="10" cy="18" r="2"/>',
    bell: '<path d="M5 17h14l-2-4V9a5 5 0 0 0-10 0v4Zm5 3h4"/>',
    chevron: '<path d="m14 6-6 6 6 6"/>',
  };
  const escape = (value) =>
    String(value).replace(
      /[&<>"']/g,
      (c) =>
        ({
          "&": "&amp;",
          "<": "&lt;",
          ">": "&gt;",
          '"': "&quot;",
          "'": "&#39;",
        })[c],
    );
  const svg = (name) =>
    `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${paths[name] || `<use href="#i-${name}"/>`}</svg>`;
  const box = (x, y, w, h) =>
    `left:${x}px;top:${y}px;width:${w}px;height:${h}px`;
  const label = (text, x, y, w, h, cls = "") =>
    `<span class="native-label ${cls}" style="${box(x, y, w, h)}">${text}</span>`;
  const symbol = (name, x, y, size = 14, cls = "") =>
    label(svg(name), x, y, size, size, `native-symbol ${cls}`);
  const button = (
    text,
    x,
    y,
    w,
    h,
    attrs = "",
    cls = "",
    illustrative = false,
  ) =>
    `<${illustrative ? "span" : 'button type="button"'} class="native-button ${cls}" style="${box(x, y, w, h)}" ${illustrative ? "" : attrs}>${text}</${illustrative ? "span" : "button"}>`;
  const time = (seconds) =>
    `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;

  function header(module, width, illustrative, hero, state) {
    const index = sections.indexOf(module);
    const status =
      !illustrative &&
      state.started &&
      !(module === "music" && state.lyrics && state.musicPlaying)
        ? `<span class="native-header-timer" style="${box(width - 164, 7, 118, 26)}"><span class="timer-dot"></span><span data-timer>${time(state.remaining)}</span><button type="button" data-timer-action aria-label="Pause focus timer preview">${svg(state.running ? "pause" : "play")}</button></span>`
        : label("Fri 30 Oct 2026", width - 232, 10, 132, 20, "native-date") +
          label("9:41", width - 92, 9, 44, 22, "native-clock");
    return `<header class="native-header">${symbol(module === "usage" ? "usage" : module === "openmaus" ? "bot" : module, 24, 13, 14, "native-muted")}${label(names[index], 46, 9, 140, 22, "native-title")}${status}${hero ? button(svg("power"), width - 44, 6, 28, 28, 'data-collapse aria-label="Collapse preview"', "native-power", illustrative) : symbol("power", width - 40, 12, 16)}</header>`;
  }

  function footer(module, width, height, illustrative, host) {
    const active = sections.indexOf(module);
    return `<nav class="native-footer" style="top:${height - 26}px;width:${width}px" ${illustrative ? 'aria-hidden="true"' : 'aria-label="Notch sections"'}>${button(svg("chevron"), 0, 0, 20, 20, `data-native-nav="${sections[(active + 7) % 8]}" data-host="${host}" aria-label="Previous notch section"`, "native-nav-arrow", illustrative)}${sections.map((section, i) => button(`<i class="${section === module ? "active" : ""}"></i>`, 0, 0, 14, 20, `data-native-nav="${section}" data-host="${host}" aria-label="Show ${names[i]} preview" aria-pressed="${section === module}"`, "native-nav-dot", illustrative)).join("")}${button(svg("chevron"), 0, 0, 20, 20, `data-native-nav="${sections[(active + 1) % 8]}" data-host="${host}" aria-label="Next notch section"`, "native-nav-arrow native-next", illustrative)}</nav>`;
  }

  function focus(s, illustrative) {
    const modes = [
      ["Focus", "target", 0, 74],
      ["Break", "coffee", 1, 72],
      ["Long", "moon", 2, 82],
    ];
    const value = illustrative ? 1499 : s.remaining;
    const mode = illustrative ? 0 : s.focusMode;
    const progress = Math.max(
      0,
      Math.min(100, (1 - value / (illustrative ? 1500 : s.total)) * 100),
    );
    let x = 24;
    return (
      modes
        .map(([name, icon, index, width]) => {
          const markup = button(
            `${index === mode ? '<i class="native-selected-dot"></i>' : ""}${svg(icon)}${name}`,
            x,
            38,
            width,
            22,
            `data-focus-mode="${index}" aria-pressed="${index === mode}"`,
            `native-mode ${index === mode ? "is-selected" : ""}`,
            illustrative,
          );
          x += width + 6;
          return markup;
        })
        .join("") +
      label(
        "Session 1/4",
        296,
        39,
        60,
        20,
        "native-small native-center native-muted",
      ) +
      `<span class="native-cycles" aria-hidden="true"><i></i><i></i><i></i><i></i></span>` +
      `<div class="native-focus-ring" style="--progress:${100 - progress}%">${svg(modes[mode][1])}</div>` +
      label(
        illustrative ? "24:59" : time(value),
        106,
        67,
        160,
        36,
        "native-big native-timer",
      ) +
      label(
        illustrative
          ? "Stay focused • Session active"
          : s.remaining === 0
            ? "Session complete"
            : s.running
              ? "Stay focused • Session active"
              : s.started
                ? "Paused • Tap to resume"
                : mode === 0
                  ? "Ready to focus"
                  : "Ready for a break",
        106,
        103,
        170,
        16,
        "native-muted native-sub native-focus-status",
      ) +
      `<span class="native-progress" style="${box(106, 126, 160, 3)}"><i style="width:${progress}%"></i></span>` +
      label(
        `${Math.round(progress)}%`,
        236,
        131,
        30,
        14,
        "native-small native-muted native-percent",
      ) +
      button(
        `${svg(!illustrative && !s.running ? "play" : "pause")}<span>${illustrative ? "Pause" : s.running ? "Pause" : s.started ? "Resume" : "Start"}</span>`,
        288,
        68,
        136,
        36,
        'data-timer-action aria-label="' +
          (s.running ? "Pause" : "Start") +
          ' focus timer preview"',
        "native-primary mini-action",
        illustrative,
      ) +
      button(
        svg("reset"),
        288,
        114,
        38,
        26,
        'data-focus-reset aria-label="Reset focus timer preview"',
        "native-secondary",
        illustrative,
      ) +
      button(
        "+5m",
        332,
        114,
        48,
        26,
        'data-focus-add aria-label="Add five minutes"',
        "native-secondary",
        illustrative,
      ) +
      button(
        svg("next"),
        386,
        114,
        38,
        26,
        'data-focus-skip aria-label="Skip to next timer session"',
        "native-secondary",
        illustrative,
      )
    );
  }

  const track = {
    title: "Merry Christmas, Please Don’t Call",
    artist: "Bleachers",
    album: "Merry Christmas, Please Don’t Call - Single",
    year: 2024,
    duration: 202,
    lyric: "Merry Christmas, please don’t call",
    artwork: "assets/bleachers-merry-christmas.jpg",
  };

  function music(s, illustrative) {
    const lyric = s.lyrics;
    const x = lyric ? 96 : 120,
      right = lyric ? 220 : 424,
      position = illustrative ? 84 : s.musicPosition;
    return (
      `<div class="native-album" style="${box(24, 44, lyric ? 56 : 80, lyric ? 56 : 80)}"><img src="${track.artwork}" alt="Merry Christmas, Please Don’t Call single artwork" width="80" height="80"/></div>` +
      label(
        `<span class="native-song-title ${s.musicPlaying ? "is-playing" : ""}" style="--title-window:${right - x}px" title="${escape(track.title)}">${track.title}</span>`,
        x,
        44,
        right - x,
        24,
        "native-title native-track-title",
      ) +
      label(track.artist, x, 70, right - x, 20, "native-sub native-muted") +
      `<span class="native-progress" style="${box(lyric ? 24 : 120, lyric ? 106 : 96, lyric ? 196 : 304, 3)}"><i data-music-progress style="width:${(position / track.duration) * 100}%"></i></span>` +
      label(
        time(position),
        lyric ? 24 : 120,
        lyric ? 112 : 102,
        54,
        14,
        "native-small native-muted native-music-elapsed",
      ) +
      label(
        time(track.duration),
        lyric ? 182 : 386,
        lyric ? 112 : 102,
        38,
        14,
        "native-small native-muted native-right",
      ) +
      button(
        svg("lyrics"),
        lyric ? 24 : 48,
        132,
        32,
        28,
        `data-lyrics aria-label="Toggle lyrics preview" aria-pressed="${lyric}"`,
        lyric ? "is-selected" : "",
        illustrative,
      ) +
      button(
        svg("previous"),
        lyric ? 72 : 208,
        132,
        32,
        28,
        'data-track="-1" aria-label="Rewind preview by 15 seconds"',
        "",
        illustrative,
      ) +
      button(
        svg(s.musicPlaying ? "pause" : "play"),
        lyric ? 118 : 254,
        128,
        36,
        36,
        `data-music-action aria-label="${s.musicPlaying ? "Pause" : "Play"} music preview"`,
        "native-primary",
        illustrative,
      ) +
      button(
        svg("next"),
        lyric ? 168 : 304,
        132,
        32,
        28,
        'data-track="1" aria-label="Forward preview by 15 seconds"',
        "",
        illustrative,
      ) +
      (lyric
        ? `<div class="native-lyrics">${label("···", 0, 0, 280, 28, "native-muted")}${label(track.lyric, 0, 28, 280, 28, "native-lyric-active")}${label("···", 0, 56, 280, 28, "native-muted")}${label("Short lyric preview · no audio", 0, 94, 280, 18, "native-small native-muted")}</div>`
        : "")
    );
  }

  function water(s, illustrative) {
    const amount = illustrative ? 750 : s.water;
    const goal = s.waterGoal;
    return (
      label(
        `${(amount / 1000).toFixed(1)} / ${(goal / 1000).toFixed(1)} L`,
        38,
        41,
        194,
        36,
        "native-big native-water-value",
      ) +
      label(
        `Today · ${Math.round((amount / goal) * 100)}% of goal`,
        38,
        81,
        194,
        19,
        "native-sub native-muted native-water-percent",
      ) +
      button(
        `${svg("water")}<span>+250 ml</span>`,
        38,
        116,
        92,
        28,
        "data-drink",
        "native-primary",
        illustrative,
      ) +
      button(
        `${svg("sliders")}Goal`,
        138,
        116,
        82,
        28,
        'data-water-goal aria-label="Change demo water goal"',
        "native-secondary",
        illustrative,
      ) +
      label("Hourly", 244, 36, 95, 20, "native-sub native-muted") +
      button(
        "<i></i>",
        374,
        35,
        36,
        20,
        'data-water-reminder role="switch" aria-checked="' +
          s.reminder +
          '" aria-label="Hourly reminder preview"',
        `native-switch ${s.reminder ? "is-on" : ""}`,
        illustrative,
      ) +
      `<div class="native-hourly" aria-label="Sample hourly water intake">${[250, 0, 250, amount - 500, 0, 0, 0, 0].map((value, i) => `<span><i style="height:${Math.max(0, Math.min(100, (value / 400) * 100))}%"></i><small>${String(i + 9).padStart(2, "0")}</small></span>`).join("")}</div>`
    );
  }

  function tasks(s, illustrative, host) {
    const offset = s.taskPage * 3;
    const inputId = `preview-task-input-${host}`;
    let content =
      label(
        `${s.taskDone.filter(Boolean).length} / ${s.taskTitles.length} done`,
        38,
        43,
        160,
        24,
        "native-sub native-muted",
      ) +
      button(
        `${svg("plus")}New task`,
        378,
        42,
        104,
        28,
        "data-task-new",
        "native-primary",
        illustrative,
      );
    if (s.taskEditing !== null) {
      return (
        content +
        `<form class="native-task-form" data-task-form><label for="${inputId}">${s.taskEditing === -1 ? "New task" : "Edit task"}</label><input id="${inputId}" name="task" maxlength="100" placeholder="What would you like to do?" value="${s.taskEditing === -1 ? "" : escape(s.taskTitles[s.taskEditing])}" required autocomplete="off"/><span>Enter to save · Esc to cancel</span><button type="button" data-task-cancel>Cancel</button><button type="submit">Save</button></form>`
      );
    }
    content += s.taskTitles.length
      ? s.taskTitles
          .slice(offset, offset + 3)
          .map((title, row) => {
            const i = offset + row;
            return `<div class="native-task-row" style="top:${76 + row * 34}px"><button type="button" data-task="${i}" aria-pressed="${s.taskDone[i]}"><i>${s.taskDone[i] ? svg("check") : ""}</i><span>${escape(title)}</span></button><button type="button" data-task-edit="${i}" aria-label="Edit ${escape(title)}">${svg("edit")}</button><button type="button" data-task-delete="${i}" aria-label="Delete ${escape(title)}">${svg("trash")}</button></div>`;
          })
          .join("")
      : label(
          "No tasks yet · Add your first task",
          38,
          102,
          444,
          30,
          "native-center native-muted",
        );
    if (s.taskTitles.length > 3) {
      if (s.taskPage > 0)
        content += button(
          svg("previous"),
          195,
          174,
          28,
          28,
          `data-task-page="${s.taskPage - 1}" aria-label="Previous task page"`,
          "",
          illustrative,
        );
      content += label(
        `${s.taskPage + 1} / ${Math.ceil(s.taskTitles.length / 3)}`,
        229,
        176,
        62,
        24,
        "native-center native-sub native-muted",
      );
      if (offset + 3 < s.taskTitles.length)
        content += button(
          svg("next"),
          297,
          174,
          28,
          28,
          `data-task-page="${s.taskPage + 1}" aria-label="Next task page"`,
          "",
          illustrative,
        );
    }
    return content;
  }

  function files() {
    return `<div class="native-dropzone">${svg("files")}<strong>Drop files here</strong><span>Drag to the notch · Click to open</span></div>`;
  }

  function maus(s, illustrative) {
    const tabs = ["Activity", "Schedule", "Usage"];
    const headings = [
      "No work in progress",
      "1 active automation",
      "Usage overview",
    ];
    const details = [
      "Latest activity from OpenMaus Bot",
      "Schedules saved in OpenMaus Bot",
      "Example bot, model, and provider usage",
    ];
    let content = tabs
      .map((name, i) =>
        button(
          name,
          38 + i * 100,
          38,
          96,
          26,
          `data-maus-tab="${i}" aria-pressed="${s.mausTab === i}"`,
          s.mausTab === i ? "native-tab is-selected" : "native-tab",
          illustrative,
        ),
      )
      .join("");
    content +=
      label(
        '<i class="native-online"></i>Connected',
        437,
        41,
        85,
        20,
        "native-sub native-muted",
      ) +
      label(headings[s.mausTab], 38, 72, 378, 20, "native-title") +
      label(details[s.mausTab], 38, 93, 416, 15, "native-small native-muted");
    if (s.mausTab === 0)
      content +=
        symbol("check", 44, 120, 14) +
        label("Agent finished responding", 69, 112, 340, 18) +
        label(
          "Research assistant · example activity",
          69,
          129,
          416,
          15,
          "native-small native-muted",
        ) +
        label("9:40", 438, 112, 84, 18, "native-right native-muted") +
        symbol("bot", 44, 154, 14) +
        label("Daily summary completed", 69, 146, 340, 18) +
        label(
          "Example automation",
          69,
          163,
          416,
          15,
          "native-small native-muted",
        );
    if (s.mausTab === 1)
      content +=
        symbol("focus", 44, 120, 14) +
        label("Daily summary", 69, 112, 300, 18) +
        label(
          "Example bot · Every day at 09:00",
          69,
          129,
          416,
          15,
          "native-small native-muted",
        ) +
        label("Tomorrow", 394, 112, 128, 18, "native-right native-muted");
    if (s.mausTab === 2)
      content +=
        label("Example bot", 38, 117, 196, 18, "native-muted") +
        label("24.8k tokens", 250, 117, 272, 18, "native-right") +
        label("Example model / provider", 38, 147, 330, 18, "native-muted") +
        label("12 requests", 378, 147, 144, 18, "native-right");
    return content;
  }

  function usage(s, illustrative) {
    const providers = ["Gemini", "Claude", "Codex", "OpenCode"];
    return (
      providers
        .map((name, i) =>
          button(
            `<img src="assets/providers/${name.toLowerCase()}.png" alt=""/>${name}`,
            24 + i * 102,
            36,
            94,
            26,
            `data-provider="${i}" aria-pressed="${s.provider === i}"`,
            s.provider === i
              ? "native-provider is-selected"
              : "native-provider",
            illustrative,
          ),
        )
        .join("") +
      label("Today's tokens", 28, 72, 160, 17, "native-sub native-muted") +
      label("24.8k", 27, 91, 166, 36, "native-big") +
      label(
        '<i class="native-online"></i>Idle · 12 req',
        28,
        136,
        168,
        18,
        "native-sub native-muted",
      ) +
      `<span class="native-divider" style="${box(210, 74, 1, 80)}"></span>` +
      label("Model", 228, 72, 190, 16, "native-sub native-muted") +
      label(
        `${providers[s.provider]} · sample model`,
        228,
        88,
        196,
        20,
        "native-strong",
      ) +
      label(
        "Input 18.2k · Output 6.6k",
        228,
        113,
        196,
        18,
        "native-sub native-muted",
      ) +
      label(
        "Total 124.8k tokens",
        228,
        135,
        196,
        18,
        "native-sub native-muted",
      ) +
      `<div class="native-token-bar"><i></i><i></i><i></i><i></i></div>`
    );
  }

  function stocks(s, illustrative) {
    const tickers = ["AAPL", "MSFT", "NVDA", "GOOGL", "BBCA", "BBRI"];
    const companies = [
      "Apple Inc.",
      "Microsoft Corporation",
      "NVIDIA Corporation",
      "Alphabet Inc.",
      "Bank Central Asia",
      "Bank Rakyat Indonesia",
    ];
    const prices = [
      "USD 187.42",
      "USD 412.18",
      "USD 138.25",
      "USD 175.64",
      "IDR 9250.00",
      "IDR 4120.00",
    ];
    const slot = 382 / tickers.length;
    return (
      tickers
        .map((ticker, i) =>
          button(
            ticker,
            34 + i * slot,
            33,
            slot - 4,
            25,
            `data-stock="${i}" aria-pressed="${s.stock === i}"`,
            s.stock === i ? "is-selected native-ticker" : "native-ticker",
            illustrative,
          ),
        )
        .join("") +
      label(companies[s.stock], 38, 62, 220, 18, "native-sub native-muted") +
      label(prices[s.stock], 38, 82, 215, 34, "native-big") +
      label("+1.24%", 38, 116, 150, 20, "native-positive") +
      label(
        "Illustrative quote · sample data",
        38,
        139,
        225,
        15,
        "native-small native-muted",
      ) +
      label("5 DAYS", 275, 61, 129, 15, "native-small native-muted") +
      `<svg class="native-stock-chart" viewBox="0 0 129 42" aria-hidden="true"><path class="native-chart-grid" d="M0 0h129M0 21h129M0 42h129"/><path d="m0 42 10-7 8 3 9-10 8 4 10-12 8 5 11-8 9 4 10-11 8 3 9-9 8 3 11-7"/></svg>`
    );
  }

  function render(module, state, { illustrative = false, hero = false } = {}) {
    const width =
      module === "tasks"
        ? 520
        : module === "openmaus" || (module === "music" && state.lyrics)
          ? 560
          : 448;
    const height = module === "tasks" || module === "openmaus" ? 232 : 196;
    const body = {
      focus,
      music,
      water,
      tasks,
      files,
      openmaus: maus,
      usage,
      stocks,
    }[module](state, illustrative, hero ? "hero" : "explorer");
    return `<div class="native-viewport" style="--native-width:${width};--native-height:${height};aspect-ratio:${width}/${height}"><div class="native-surface" data-native-module="${module}" style="width:${width}px;height:${height}px"><span class="native-ambient" aria-hidden="true"></span>${header(module, width, illustrative, hero, state)}${body}${footer(module, width, height, illustrative, hero ? "hero" : "explorer")}</div></div>`;
  }

  const resize = new ResizeObserver((entries) =>
    entries.forEach(({ target, contentRect }) =>
      target.style.setProperty(
        "--native-scale",
        contentRect.width /
          Number(target.style.getPropertyValue("--native-width")),
      ),
    ),
  );
  function fit() {
    resize.disconnect();
    document.querySelectorAll(".native-viewport").forEach((element) => {
      element.style.setProperty(
        "--native-scale",
        element.clientWidth /
          Number(element.style.getPropertyValue("--native-width")),
      );
      resize.observe(element);
    });
  }

  function mascot() {
    let dots = "";
    for (let row = -6; row <= 6; row++)
      for (let col = -27; col <= 27; col++) {
        const dx = (col + (row & 1) * 0.5) * 3.5,
          dy = row * 3.5,
          radius = Math.hypot(dx, dy) / 16;
        const ring = Math.max(0, 1 - Math.abs((radius - 1) / 0.34)),
          hole = Math.max(0, Math.min(1, (radius - 0.52) / 0.22));
        if (94 + dx >= 0 && 94 + dx <= 188 && 20 + dy >= 0 && 20 + dy <= 40)
          dots += `<circle cx="${94 + dx}" cy="${20 + dy}" r="${(0.5 + 0.24 * ring) * 1.0}" opacity="${hole * (0.18 + 0.38 * ring)}"/>`;
      }
    const stripes = [-8, 0, 8]
      .map(
        (x, i) =>
          `<path d="M${94 + x - 2.2} 3h4.4C${94 + x + 2.2} 3 ${94 + x + 1.1} ${3 + [6.5, 9, 6.5][i]} ${94 + x} ${3 + [6.5, 9, 6.5][i]}C${94 + x - 1.1} ${3 + [6.5, 9, 6.5][i]} ${94 + x - 2.2} 3 ${94 + x - 2.2} 3Z"/>`,
      )
      .join("");
    const eyes = [-1, 1]
      .map(
        (side) =>
          `<ellipse class="native-eye" cx="${94 + side * 17}" cy="23" rx="6.4" ry="7.2"/>`,
      )
      .join("");
    const pupils = [-1, 1]
      .map(
        (side) =>
          `<ellipse cx="${94 + side * 17}" cy="23" rx="2.5" ry="4.6"/><ellipse fill="white" cx="${94 + side * 17 - 0.65}" cy="21.3" rx=".8" ry="1.1"/>`,
      )
      .join("");
    const whiskers = [-1, 1]
      .flatMap((side) =>
        [
          [24, 20],
          [27, 27],
          [30, 34],
        ].map(
          ([y, end]) =>
            `<path d="M${94 + side * 38} ${y}L${94 + side * 57} ${end}"/>`,
        ),
      )
      .join("");
    return `<svg class="tiger-eyes" viewBox="0 0 188 40" aria-hidden="true"><g fill="#8cc7ff">${dots}</g><g class="native-face-features"><g fill="#fff">${stripes}${eyes}</g><g class="pupils" fill="#1a1714">${pupils}</g><g stroke="#fff" stroke-width="1.25" stroke-linecap="round">${whiskers}</g></g></svg>`;
  }

  window.NocanPreview = { render, fit, mascot, track };
})();
