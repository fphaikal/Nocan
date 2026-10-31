(() => {
  "use strict";

  const $ = (selector, root = document) => root.querySelector(selector);
  const $$ = (selector, root = document) => [
    ...root.querySelectorAll(selector),
  ];
  const icon = (name, className = "") =>
    `<svg class="icon ${className}" aria-hidden="true"><use href="#i-${name}"/></svg>`;
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
  const systemTheme = window.matchMedia("(prefers-color-scheme: dark)");
  const state = {
    mode: "music",
    module: "music",
    remaining: 1500,
    running: false,
    started: false,
    musicPlaying: true,
    water: 500,
    taskDone: [false, false, false],
    taskTitles: [
      "Send the finished work",
      "Review the latest design",
      "Prepare meeting notes",
    ],
    taskEditing: null,
    taskPage: 0,
    focusMode: 0,
    total: 1500,
    musicPosition: 57.36,
    lyricCues: NocanLyrics.defaultCues(),
    lyrics: true,
    waterGoal: 2000,
    reminder: true,
    mausTab: 0,
    provider: 0,
    stock: 0,
    open: false,
    scene: "focus",
    theme: "light",
  };
  let motionOff = reducedMotion.matches;
  let motionContext;
  let toastTimeout;
  let lastTick = Date.now();
  let lastMusicTick = Date.now();
  let lastLyricIndex = -1;
  let lyricsRequest = 0;
  let lyricsAbort;

  try {
    const savedMotion = localStorage.getItem("nocan-site-motion");
    if (savedMotion !== null) motionOff = savedMotion === "off";
  } catch {
    /* Browser storage is optional. */
  }

  const modules = {
    music: {
      title: "Music",
      icon: "music",
      kicker: "A soundtrack within reach",
      heading: "Stay in the song.",
      description:
        "Playback controls, album artwork, and synchronized lyrics from the music playing on your Windows desktop.",
    },
    focus: {
      title: "Focus",
      icon: "focus",
      kicker: "Make time for what matters",
      heading: "Your focus, in view.",
      description:
        "Work and break sessions, a timer that changes with you, and a little extra time when you need it.",
    },
    water: {
      title: "Water",
      icon: "water",
      kicker: "A small moment for yourself",
      heading: "One sip at a time.",
      description:
        "Gentle in-notch reminders, a little celebration when you log a drink, and a daily history that stays saved.",
    },
    files: {
      title: "Files",
      icon: "files",
      kicker: "Put it somewhere handy",
      heading: "A little shelf for later.",
      description:
        "Drop files into the notch, then open them when you need them. A small shortcut for the things you keep reaching for.",
    },
    tasks: {
      title: "Tasks",
      icon: "tasks",
      kicker: "A little less on your mind",
      heading: "Make a little progress.",
      description:
        "Add, edit, complete, and delete tasks right in the notch. In this preview, click a task to check it off.",
    },
    openmaus: {
      title: "OpenMaus",
      icon: "bot",
      kicker: "Keep an eye on your agents",
      heading: "Know when it is done.",
      description:
        "Local agent activity, automation schedules, and completion alerts. See what is running without leaving your work.",
    },
    usage: {
      title: "AI Usage",
      icon: "usage",
      kicker: "Your local activity, at a glance",
      heading: "A clearer picture.",
      description:
        "See available local usage records for Gemini, Claude, Codex, and OpenCode. Usage depends on the records available on your machine.",
    },
    stocks: {
      title: "Stocks",
      icon: "stocks",
      kicker: "A quick glance at your watchlist",
      heading: "Keep your tickers close.",
      description:
        "Follow up to six IDX and global tickers with price changes and recent history. Live market data in the app may be delayed.",
    },
  };

  function formatTime(seconds) {
    const whole = Math.max(0, Math.floor(seconds));
    return `${String(Math.floor(whole / 60)).padStart(2, "0")}:${String(whole % 60).padStart(2, "0")}`;
  }

  function timerColor() {
    const mix = (a, b, t) =>
      `rgb(${a.map((channel, i) => Math.round((channel + (b[i] - channel) * Math.max(0, Math.min(1, t))) * 255)).join(" ")})`;
    const ratio = state.remaining / state.total;
    if (state.remaining === 0) return "#144a8f";
    if (!state.running) return "#303640";
    if (state.remaining <= 60)
      return mix(
        [0.45, 0.1, 0.13],
        [0.62, 0.07, 0.12],
        1 - state.remaining / 60,
      );
    if (ratio <= 0.25)
      return mix([0.39, 0.29, 0.06], [0.48, 0.2, 0.05], 1 - ratio / 0.25);
    return mix([0.06, 0.3, 0.2], [0.26, 0.3, 0.1], (1 - ratio) / 0.75);
  }

  function updateTimer() {
    $$("[data-timer]").forEach((element) => {
      element.textContent =
        state.remaining === 0 ? "Done" : formatTime(state.remaining);
    });
    $$("[data-timer-action]").forEach((button) => {
      button.innerHTML =
        icon(state.running ? "pause" : "play") +
        (button.classList.contains("mini-action")
          ? `<span>${state.running ? "Pause" : state.started ? "Resume" : "Start"}</span>`
          : "");
      button.setAttribute(
        "aria-label",
        `${state.running ? "Pause" : "Start"} focus timer preview`,
      );
    });
    const currentLyric = NocanLyrics.at(state.lyricCues, state.musicPosition);
    const hasLyric =
      state.mode === "music" &&
      state.musicPlaying &&
      state.lyrics &&
      Boolean(currentLyric.text);
    const detached = hasLyric && state.started;
    $(".floating-timer").hidden = !detached || state.open;
    $(".floating-timer").style.backgroundColor = timerColor();
    $(".compact-timer").style.backgroundColor = timerColor();
    $$(".native-header-timer").forEach(
      (element) => (element.style.backgroundColor = timerColor()),
    );
    $(".compact-timer").hidden = !state.started || state.open || detached;
    $(".native-mascot").hidden = state.started || hasLyric;
    $(".compact-lyric").hidden = !hasLyric || state.open;
    $(".notch-demo").classList.toggle("has-lyrics", hasLyric && !state.open);
    $(".notch-demo").classList.toggle(
      "has-detached-timer",
      detached && !state.open,
    );
    $(".notch-demo").style.setProperty(
      "--lyric-width",
      `${NocanPreview.lyricWidth(currentLyric.text)}px`,
    );
    $$(
      ".hero-panel-content .native-timer, .explorer-preview .native-timer",
    ).forEach((element) => (element.textContent = formatTime(state.remaining)));
    const percent = Math.max(
      0,
      Math.min(100, (1 - state.remaining / state.total) * 100),
    );
    $$(
      ".hero-panel-content .native-focus-ring, .explorer-preview .native-focus-ring",
    ).forEach((element) =>
      element.style.setProperty("--progress", `${100 - percent}%`),
    );
    $$(
      ".hero-panel-content .native-percent, .explorer-preview .native-percent",
    ).forEach((element) => (element.textContent = `${Math.round(percent)}%`));
    $$(
      '.native-surface[data-native-module="focus"] .native-progress i',
    ).forEach((element) => {
      if (!element.closest(".story-notch")) element.style.width = `${percent}%`;
    });
    $$(
      ".hero-panel-content .native-focus-status, .explorer-preview .native-focus-status",
    ).forEach(
      (element) =>
        (element.textContent =
          state.remaining === 0
            ? "Session complete"
            : state.running
              ? "Stay focused • Session active"
              : state.started
                ? "Paused • Tap to resume"
                : state.focusMode === 0
                  ? "Ready to focus"
                  : "Ready for a break"),
    );
  }

  function syncMusic(force = false) {
    const current = NocanLyrics.at(state.lyricCues, state.musicPosition);
    $$(".native-music-elapsed").forEach(
      (element) => (element.textContent = formatTime(state.musicPosition)),
    );
    $$("[data-music-progress]").forEach(
      (element) =>
        (element.style.width = `${(state.musicPosition / NocanPreview.track.duration) * 100}%`),
    );
    $$("[data-music-position]").forEach(
      (element) =>
        (element.textContent = `${formatTime(state.musicPosition)} / 03:22`),
    );
    const seek = $("#music-seek");
    if (document.activeElement !== seek) seek.value = state.musicPosition;
    seek.setAttribute(
      "aria-valuetext",
      `${formatTime(state.musicPosition)} of 03:22`,
    );
    $$(".native-lyric-track").forEach(
      (element) =>
        (element.style.transform = `translateY(${28 - current.index * 28}px)`),
    );
    $$("[data-cue-index]").forEach((element) =>
      element.classList.toggle(
        "is-active",
        Number(element.dataset.cueIndex) === current.index,
      ),
    );
    $$("[data-compact-line]").forEach(
      (element) => (element.textContent = current.text),
    );
    if (force || current.index !== lastLyricIndex) {
      const before = captureNotch($(".notch-demo"));
      lastLyricIndex = current.index;
      updateTimer();
      morphNotch($(".notch-demo"), before);
      if (state.scene === "music") renderStory("music");
      if (window.gsap && !motionOff && current.text)
        gsap.fromTo(
          $$("[data-compact-line]"),
          { y: 6, opacity: 0 },
          {
            y: 0,
            opacity: 1,
            duration: 0.28,
            ease: "power2.out",
            overwrite: true,
          },
        );
    }
  }

  function notchFrame(host) {
    return host.matches(".notch-demo")
      ? host
      : host.querySelector(".native-compact-lyrics") ||
          host.querySelector(".native-viewport");
  }

  function captureNotch(host) {
    const frame = notchFrame(host);
    return frame ? frame.getBoundingClientRect() : null;
  }

  function morphNotch(host, before, changed = false) {
    const frame = notchFrame(host);
    if (!frame || !before || !window.gsap || motionOff) return;
    gsap.killTweensOf(frame);
    gsap.set(frame, { scaleX: 1, scaleY: 1 });
    const after = frame.getBoundingClientRect();
    if (!after.width || !after.height) return;
    const resized =
      Math.abs(before.width - after.width) > 1 ||
      Math.abs(before.height - after.height) > 1;
    if (resized) {
      gsap.fromTo(
        frame,
        {
          scaleX: before.width / after.width,
          scaleY: before.height / after.height,
          transformOrigin: "50% 0%",
        },
        {
          scaleX: 1,
          scaleY: 1,
          duration: 0.78,
          ease: "elastic.out(1,.65)",
          overwrite: true,
          clearProps: "transform,transformOrigin",
        },
      );
    }
    if (changed || resized) {
      const content = host.matches(".notch-demo")
        ? [host.querySelector(state.open ? ".hero-demo-panel" : ".notch-face")]
        : [
            ...host.querySelectorAll(
              ".native-header, .native-label, .native-button, .native-focus-ring, .native-task-row, .native-hourly, .native-lyrics, .native-album, .native-footer, .native-compact-lyrics > span",
            ),
          ];
      gsap.fromTo(
        content.filter(Boolean),
        { opacity: 0 },
        {
          opacity: 1,
          duration: 0.28,
          delay: resized ? 0.12 : 0,
          overwrite: true,
          clearProps: "opacity",
        },
      );
    }
  }

  function renderHero() {
    const host = $(".notch-demo");
    const before = captureNotch(host);
    const changed = host.dataset.demoMode !== state.mode;
    $(".notch-demo").dataset.demoMode = state.mode;
    $(".notch-demo").style.setProperty(
      "--expanded-width",
      `${state.mode === "tasks" ? 520 : state.mode === "openmaus" || (state.mode === "music" && state.lyrics) ? 560 : 448}px`,
    );
    $(".hero-panel-content").innerHTML = NocanPreview.render(
      state.mode,
      state,
      { hero: true },
    );
    NocanPreview.fit();
    $$("[data-hero-mode]").forEach((button) =>
      button.setAttribute(
        "aria-pressed",
        String(button.dataset.heroMode === state.mode),
      ),
    );
    updateTimer();
    syncMusic();
    morphNotch(host, before, changed);
  }

  function openNotch(open = true) {
    const before = captureNotch($(".notch-demo"));
    state.open = open;
    $(".notch-demo").classList.toggle("is-open", open);
    $(".notch-face").setAttribute("aria-expanded", String(open));
    $(".notch-face").setAttribute(
      "aria-label",
      `${open ? "Collapse" : "Expand"} Nocan interactive preview`,
    );
    $(".hero-demo-panel").inert = !open;
    requestAnimationFrame(() => NocanPreview.fit());
    updateTimer();
    NocanPreview.fit();
    morphNotch($(".notch-demo"), before, true);
  }

  function renderModule(module, animate = true) {
    const before = captureNotch($(".explorer-preview"));
    state.module = module;
    const data = modules[module];
    $$("[data-module]").forEach((button) => {
      const active = button.dataset.module === module;
      button.setAttribute("aria-selected", String(active));
      button.tabIndex = active ? 0 : -1;
    });
    const panel = $(".module-panel");
    panel.setAttribute("aria-labelledby", `module-${module}`);
    $(".explorer-preview").innerHTML = NocanPreview.render(module, state);
    NocanPreview.fit();
    $(".module-kicker").textContent = data.kicker;
    $(".module-description h3").textContent = data.heading;
    $(".module-description > p").textContent = data.description;
    $(".module-preview-tools").hidden = module !== "focus";
    $(".module-music-credit").hidden = module !== "music";
    $(".music-timeline-controls").hidden = module !== "music";
    morphNotch($(".explorer-preview"), before, animate);
    updateTimer();
    syncMusic();
  }

  function renderStory(scene) {
    if (!modules[scene]) return;
    state.scene = scene;
    const data = modules[scene];
    const panel = $(".story-notch");
    const before = captureNotch(panel);
    $(".flow-stage").dataset.scene = scene;
    panel.innerHTML =
      scene === "music"
        ? NocanPreview.compactPreview(state)
        : NocanPreview.render(scene, state, { illustrative: true });
    NocanPreview.fit();
    const index = ["focus", "music", "water"].indexOf(scene);
    $(".scene-label").textContent = [
      "A little focus",
      "A little soundtrack",
      "A little care",
    ][index];
    $(".scene-count").textContent = `0${index + 1} / 03`;
    if (motionOff)
      $(".scene-track span").style.transform = `scaleX(${(index + 1) / 3})`;
    $(".scene-disclaimer").textContent =
      scene === "music"
        ? "Bleachers · Merry Christmas, Please Don’t Call · 2024 · 3:22 · Preview without audio"
        : "Illustrative preview of the desktop experience.";
    if (window.gsap && !motionOff) {
      morphNotch(panel, before, true);
      if (scene === "music" && $(".native-scene-chip"))
        gsap.fromTo(
          $(".native-scene-chip"),
          { x: -26, opacity: 0 },
          { x: 0, opacity: 1, duration: 0.45, delay: 0.18, ease: "power3.out" },
        );
    }
    $$(".flow-chapter").forEach((chapter) =>
      chapter.classList.toggle("is-active", chapter.dataset.chapter === scene),
    );
  }

  function setTheme(theme) {
    state.theme = theme;
    const dark =
      theme === "dark" || (theme === "system" && systemTheme.matches);
    $("#settings-screenshot").src =
      `images/settings-${dark ? "dark" : "light"}.png`;
    $("#settings-screenshot").alt =
      `Actual Nocan appearance settings in ${dark ? "dark" : "light"} mode, with theme, accent, font, and opacity controls`;
    $$("[data-preview-theme]").forEach((button) =>
      button.setAttribute(
        "aria-pressed",
        String(button.dataset.previewTheme === theme),
      ),
    );
  }

  function showWaterConfirmation() {
    const confirmation = $(".water-confirmation");
    confirmation.textContent =
      state.water >= 2000
        ? "Daily goal reached. A little win!"
        : "+250 ml. A little closer.";
    confirmation.classList.add("is-visible");
    $$('.native-surface[data-native-module="water"]').forEach((element) => {
      element.classList.remove("is-drinking");
      requestAnimationFrame(() => element.classList.add("is-drinking"));
    });
    clearTimeout(toastTimeout);
    toastTimeout = setTimeout(
      () => confirmation.classList.remove("is-visible"),
      2200,
    );
    if (window.gsap && !motionOff)
      gsap.fromTo(
        $$('.native-surface[data-native-module="water"] .native-water-value'),
        { scale: 1.08 },
        { scale: 1, duration: 0.55, ease: "elastic.out(1, 0.45)" },
      );
  }

  function setMotion(off) {
    motionOff = off;
    document.documentElement.dataset.motion = off ? "off" : "on";
    const button = $(".motion-button");
    button.setAttribute("aria-pressed", String(off));
    button.setAttribute(
      "aria-label",
      off ? "Enable animations" : "Pause animations",
    );
    $("span", button).textContent = off ? "Motion off" : "Motion on";
    if (motionContext) {
      motionContext.revert();
      motionContext = null;
    }
    if (window.gsap && window.ScrollTrigger && !off) {
      gsap.registerPlugin(ScrollTrigger);
      motionContext = gsap.context(() => {
        const media = gsap.matchMedia();
        gsap
          .timeline({ defaults: { ease: "power3.out" } })
          .from(".hero-intro", { y: 12, opacity: 0, duration: 0.7 })
          .from(
            ".hero h1 > span",
            { y: 40, opacity: 0, duration: 1, stagger: 0.12 },
            "-=.5",
          )
          .from(
            ".hero-description, .hero-actions",
            { y: 17, opacity: 0, duration: 0.75, stagger: 0.1 },
            "-=.6",
          )
          .from(
            ".desktop-demo",
            { y: 42, scale: 0.97, opacity: 0, duration: 1.2 },
            "-=.55",
          );
        gsap.to(".scroll-progress", {
          scaleX: 1,
          ease: "none",
          scrollTrigger: { start: 0, end: "max", scrub: 0.2 },
        });
        gsap.fromTo(
          ".scene-track span",
          { scaleX: 0 },
          {
            scaleX: 1,
            ease: "none",
            scrollTrigger: {
              trigger: ".flow-story",
              start: "top 58%",
              end: "bottom 58%",
              scrub: 0.35,
            },
          },
        );
        $$(".reveal").forEach((element) =>
          gsap.from(element, {
            y: 30,
            opacity: 0,
            duration: 0.85,
            ease: "power3.out",
            scrollTrigger: { trigger: element, start: "top 88%", once: true },
          }),
        );
        gsap.from(".settings-figure", {
          y: 45,
          rotation: 2,
          opacity: 0.4,
          duration: 1,
          scrollTrigger: {
            trigger: ".settings-section",
            start: "top 75%",
            end: "center 60%",
            scrub: 1,
          },
        });
        media.add("(min-width: 1000px)", () => {
          ScrollTrigger.create({
            trigger: ".flow-visual",
            start: "top 134px",
            endTrigger: ".flow-story",
            end: "bottom 85%",
            pin: true,
            pinSpacing: false,
            anticipatePin: 1,
          });
          gsap.to(".desktop-screen", {
            yPercent: 8,
            ease: "none",
            scrollTrigger: {
              trigger: ".desktop-demo",
              start: "top 45%",
              end: "bottom top",
              scrub: 1,
            },
          });
        });
        $$(".flow-chapter").forEach((chapter) =>
          ScrollTrigger.create({
            trigger: chapter,
            start: "top 58%",
            end: "bottom 58%",
            onToggle: (trigger) => {
              if (trigger.isActive) renderStory(chapter.dataset.chapter);
            },
          }),
        );
        return () => media.revert();
      });
      ScrollTrigger.refresh();
    }
    if (off) {
      if (window.gsap) {
        gsap.killTweensOf(
          ".story-notch, .native-viewport, .notch-demo, .native-compact-lyrics, .native-scene-chip, .pupils, .explorer-preview",
        );
        gsap.set(
          ".native-viewport, .notch-demo, .native-compact-lyrics, .native-scene-chip",
          { clearProps: "transform,transformOrigin,opacity" },
        );
        gsap.set(".story-notch, .explorer-preview", { opacity: 1, y: 0 });
      }
      renderStory(state.scene);
      $$(".flow-chapter").forEach((chapter) =>
        chapter.classList.add("is-active"),
      );
      $(".scroll-progress").style.transform = "scaleX(0)";
    }
  }

  $(".notch-face").addEventListener("click", () => openNotch(!state.open));
  $$("[data-hero-mode]").forEach((button) =>
    button.addEventListener("click", () => {
      state.mode = button.dataset.heroMode;
      renderHero();
      openNotch(true);
    }),
  );
  $$("[data-module]").forEach((button) =>
    button.addEventListener("click", () => renderModule(button.dataset.module)),
  );
  $(".module-list").addEventListener("keydown", (event) => {
    const buttons = $$("[data-module]");
    let index = buttons.indexOf(document.activeElement);
    if (index < 0) return;
    if (["ArrowDown", "ArrowRight"].includes(event.key))
      index = (index + 1) % buttons.length;
    else if (["ArrowUp", "ArrowLeft"].includes(event.key))
      index = (index - 1 + buttons.length) % buttons.length;
    else if (event.key === "Home") index = 0;
    else if (event.key === "End") index = buttons.length - 1;
    else return;
    event.preventDefault();
    buttons[index].focus();
    renderModule(buttons[index].dataset.module);
  });
  $$("[data-preview-theme]").forEach((button) =>
    button.addEventListener("click", () =>
      setTheme(button.dataset.previewTheme),
    ),
  );
  systemTheme.addEventListener("change", () => {
    if (state.theme === "system") setTheme("system");
  });
  $("#music-seek").addEventListener("input", (event) => {
    state.musicPosition = Number(event.target.value);
    lastMusicTick = Date.now();
    syncMusic();
  });
  const replaySong = $("[data-replay-song], [data-replay-excerpt]");
  replaySong.textContent = "Replay song";
  replaySong.dataset.replaySong = "";
  const retryLyrics =
    $("[data-retry-lyrics]") || document.createElement("button");
  if (!retryLyrics.parentElement) {
    retryLyrics.type = "button";
    retryLyrics.dataset.retryLyrics = "";
    retryLyrics.textContent = "Retry lyrics";
    retryLyrics.hidden = true;
    $(".music-timeline-controls").append(retryLyrics);
  }
  replaySong.addEventListener("click", () => {
    state.musicPosition = 0;
    state.musicPlaying = true;
    lastLyricIndex = -1;
    lastMusicTick = Date.now();
    refreshPreviews();
    syncMusic(true);
  });
  $("[data-lrc-file]").addEventListener("change", async (event) => {
    const file = event.target.files?.[0];
    if (!file) return;
    try {
      if (file.size > 1024 * 1024)
        throw new Error("Choose an LRC file smaller than 1 MB.");
      const cues = NocanLyrics.parse(await file.text());
      ++lyricsRequest;
      lyricsAbort?.abort();
      state.lyricCues = cues;
      state.musicPosition = state.lyricCues.find((cue) => cue.text).time;
      state.musicPlaying = true;
      state.lyrics = true;
      lastLyricIndex = -1;
      lastMusicTick = Date.now();
      $(".lyrics-source-status").textContent =
        "Local LRC loaded. Lyrics follow playback, pause, and seeking. Nothing is uploaded.";
      $(".lyrics-source-status").dataset.state = "ready";
      $(".lyrics-source-status").dataset.source = "local";
      $("[data-retry-lyrics]").hidden = true;
      refreshPreviews();
      syncMusic(true);
    } catch (error) {
      $(".lyrics-source-status").textContent = error.message;
    }
    event.target.value = "";
  });
  reducedMotion.addEventListener("change", () => {
    let saved = null;
    try {
      saved = localStorage.getItem("nocan-site-motion");
    } catch {}
    if (saved === null) setMotion(reducedMotion.matches);
  });
  $(".motion-button").addEventListener("click", () => {
    setMotion(!motionOff);
    try {
      localStorage.setItem("nocan-site-motion", motionOff ? "off" : "on");
    } catch {}
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && state.open) {
      if (event.target.closest("[data-task-form]")) return;
      openNotch(false);
      if ($(".notch-demo").contains(document.activeElement))
        $(".notch-face").focus({ preventScroll: true });
    }
  });

  document.addEventListener("click", (event) => {
    if (event.target.closest("[data-collapse]")) {
      openNotch(false);
      $(".notch-face").focus({ preventScroll: true });
      return;
    }
    const nav = event.target.closest("[data-native-nav]");
    if (nav) {
      if (nav.dataset.host === "hero") {
        state.mode = nav.dataset.nativeNav;
        renderHero();
        openNotch(true);
      } else renderModule(nav.dataset.nativeNav);
      return;
    }
    const focusHost = event.target.closest(
      ".hero-panel-content, .explorer-preview, .floating-timer",
    );
    const activeControl = [
      "data-timer-action",
      "data-final-minute",
      "data-music-action",
      "data-focus-reset",
      "data-focus-add",
      "data-focus-skip",
      "data-focus-mode",
      "data-lyrics",
      "data-track",
      "data-water-goal",
      "data-water-reminder",
      "data-maus-tab",
      "data-provider",
      "data-stock",
    ].find((attribute) => event.target.closest(`[${attribute}]`));
    if (event.target.closest("[data-timer-action]")) {
      if (state.remaining === 0) state.remaining = state.total;
      state.started = true;
      state.running = !state.running;
      lastTick = Date.now();
      renderHero();
      if (state.module === "focus") renderModule("focus", false);
    }
    if (event.target.closest("[data-final-minute]")) {
      state.remaining = 60;
      state.started = true;
      state.running = true;
      lastTick = Date.now();
      renderHero();
      if (state.module === "focus") renderModule("focus", false);
    }
    if (event.target.closest("[data-music-action]")) {
      state.musicPlaying = !state.musicPlaying;
      renderHero();
      if (state.module === "music") renderModule("music", false);
    }
    if (event.target.closest("[data-drink]")) {
      state.water = Math.min(state.water + 250, 4000);
      $$(".native-water-value").forEach(
        (element) =>
          (element.textContent = `${(state.water / 1000).toFixed(1)} / ${(state.waterGoal / 1000).toFixed(1)} L`),
      );
      $$(".native-water-percent").forEach(
        (element) =>
          (element.textContent = `Today · ${Math.round((state.water / state.waterGoal) * 100)}% of goal`),
      );
      $$(".native-hourly > span:nth-child(4) i").forEach(
        (element) =>
          (element.style.height = `${Math.min(100, Math.max(0, ((state.water - 500) / 400) * 100))}%`),
      );
      $$("[data-water]").forEach(
        (element) =>
          (element.textContent = (state.water / 1000)
            .toFixed(2)
            .replace(/0$/, "")),
      );
      $$(".water-fill").forEach((element) =>
        element.style.setProperty(
          "--water-level",
          `${Math.min((state.water / 2000) * 100, 100)}%`,
        ),
      );
      const button = event.target.closest("[data-drink]");
      const oldText = button.textContent;
      button.textContent =
        state.water >= 2000 ? "Goal reached!" : "Water logged!";
      setTimeout(() => {
        if (button.isConnected) button.textContent = oldText;
      }, 1800);
      showWaterConfirmation();
    }
    const task = event.target.closest("[data-task]");
    if (task) {
      const index = Number(task.dataset.task);
      state.taskDone[index] = !state.taskDone[index];
      refreshPreviews();
      $(`[data-task="${index}"]`, focusHost)?.focus({ preventScroll: true });
    }
    if (event.target.closest("[data-focus-reset]")) {
      state.remaining = state.total;
      state.started = false;
      state.running = false;
      refreshPreviews();
    }
    if (event.target.closest("[data-focus-add]")) {
      state.remaining += 300;
      state.total = Math.max(state.total, state.remaining);
      refreshPreviews();
    }
    const mode = event.target.closest("[data-focus-mode]");
    if (mode || event.target.closest("[data-focus-skip]")) {
      state.focusMode = mode
        ? Number(mode.dataset.focusMode)
        : (state.focusMode + 1) % 3;
      state.total = [1500, 300, 900][state.focusMode];
      state.remaining = state.total;
      state.started = false;
      state.running = false;
      refreshPreviews();
    }
    if (event.target.closest("[data-lyrics]")) {
      state.lyrics = !state.lyrics;
      refreshPreviews();
    }
    const track = event.target.closest("[data-track]");
    if (track) {
      state.musicPosition = Math.max(
        0,
        Math.min(
          NocanPreview.track.duration,
          state.musicPosition + Number(track.dataset.track) * 15,
        ),
      );
      refreshPreviews();
    }
    if (event.target.closest("[data-water-goal]")) {
      state.waterGoal = state.waterGoal === 2000 ? 3000 : 2000;
      refreshPreviews();
    }
    if (event.target.closest("[data-water-reminder]")) {
      state.reminder = !state.reminder;
      refreshPreviews();
    }
    for (const [attribute, key] of [
      ["data-maus-tab", "mausTab"],
      ["data-provider", "provider"],
      ["data-stock", "stock"],
    ]) {
      const control = event.target.closest(`[${attribute}]`);
      if (control) {
        state[key] = Number(control.getAttribute(attribute));
        refreshPreviews();
      }
    }
    if (event.target.closest("[data-task-new]")) {
      state.taskEditing = -1;
      state.taskPage = 0;
      refreshPreviews();
      $("input", focusHost)?.focus({ preventScroll: true });
    }
    const edit = event.target.closest("[data-task-edit]");
    if (edit) {
      state.taskEditing = Number(edit.dataset.taskEdit);
      refreshPreviews();
      $("input", focusHost)?.focus({ preventScroll: true });
    }
    const remove = event.target.closest("[data-task-delete]");
    if (remove) {
      const index = Number(remove.dataset.taskDelete);
      state.taskTitles.splice(index, 1);
      state.taskDone.splice(index, 1);
      state.taskPage = Math.min(
        state.taskPage,
        Math.max(0, Math.ceil(state.taskTitles.length / 3) - 1),
      );
      refreshPreviews();
      $("[data-task-new]", focusHost)?.focus({ preventScroll: true });
    }
    const page = event.target.closest("[data-task-page]");
    if (page) {
      state.taskPage = Number(page.dataset.taskPage);
      refreshPreviews();
    }
    if (event.target.closest("[data-task-cancel]")) {
      state.taskEditing = null;
      refreshPreviews();
    }
    if (focusHost && activeControl)
      $(`[${activeControl}]`, focusHost)?.focus({ preventScroll: true });
  });

  function refreshPreviews() {
    renderHero();
    renderModule(state.module, false);
  }
  document.addEventListener("submit", (event) => {
    if (!event.target.matches("[data-task-form]")) return;
    event.preventDefault();
    const value = event.target.elements.task.value.trim();
    if (!value) {
      event.target.elements.task.focus();
      return;
    }
    if (state.taskEditing === -1) {
      state.taskTitles.unshift(value);
      state.taskDone.unshift(false);
    } else state.taskTitles[state.taskEditing] = value;
    state.taskEditing = null;
    refreshPreviews();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && event.target.closest("[data-task-form]")) {
      state.taskEditing = null;
      refreshPreviews();
    }
  });

  $("#desktop-demo").addEventListener("pointermove", (event) => {
    if (motionOff || event.pointerType === "touch" || !window.gsap) return;
    const bounds = $(".tiger-eyes").getBoundingClientRect();
    gsap.to(".pupils", {
      x: gsap.utils.clamp(
        -2,
        2,
        (event.clientX - bounds.left - bounds.width / 2) / 55,
      ),
      y: gsap.utils.clamp(
        -1.5,
        1.5,
        (event.clientY - bounds.top - bounds.height / 2) / 55,
      ),
      duration: 0.3,
      overwrite: true,
    });
  });
  $("#desktop-demo").addEventListener("pointerleave", () => {
    if (window.gsap) gsap.to(".pupils", { x: 0, y: 0, duration: 0.4 });
  });

  $("#copy-command").addEventListener("click", async () => {
    const feedback = $(".copy-feedback");
    try {
      await navigator.clipboard.writeText(
        "git clone https://github.com/fphaikal/Nocan.git",
      );
      feedback.textContent = "Copied. A little project to make your own.";
      $("#copy-command span").textContent = "Copied";
    } catch {
      feedback.textContent =
        "Select the command to copy it, or open the installation guide.";
    }
    setTimeout(() => {
      $("#copy-command span").textContent = "Copy";
      feedback.textContent = "";
    }, 3500);
  });

  async function loadSongLyrics() {
    const request = ++lyricsRequest;
    lyricsAbort?.abort();
    const controller = new AbortController();
    lyricsAbort = controller;
    const status = $(".lyrics-source-status");
    status.textContent = "Loading time-synced lyrics from LRCLIB…";
    status.dataset.state = "loading";
    status.dataset.source = "lrclib";
    $("[data-retry-lyrics]").hidden = true;
    const timeout = setTimeout(() => controller.abort(), 8000);
    try {
      if (typeof NocanLyrics.load !== "function")
        await import("./lyrics-timeline.js?v=lyrics-api-2");
      const result = await NocanLyrics.load(NocanPreview.track, {
        signal: controller.signal,
      });
      if (request !== lyricsRequest) return;
      state.lyricCues = result.cues;
      lastLyricIndex = -1;
      status.textContent = "LRCLIB · Time-synced lyrics · Silent preview";
      status.dataset.state = "ready";
      refreshPreviews();
      syncMusic(true);
    } catch (error) {
      if (request !== lyricsRequest) return;
      status.textContent =
        error.name === "AbortError"
          ? "Lyrics took too long to load. Retry or load a local LRC."
          : "Lyrics could not be loaded. Retry or load a local LRC.";
      status.dataset.state = "error";
      $("[data-retry-lyrics]").hidden = false;
    } finally {
      clearTimeout(timeout);
    }
  }
  $("[data-retry-lyrics]").addEventListener("click", loadSongLyrics);

  async function findRelease() {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 4500);
    try {
      const response = await fetch(
        "https://api.github.com/repos/fphaikal/Nocan/releases?per_page=5",
        {
          signal: controller.signal,
          headers: { Accept: "application/vnd.github+json" },
        },
      );
      if (!response.ok) return;
      const releases = await response.json();
      const release = Array.isArray(releases)
        ? releases.find((item) => !item.draft && !item.prerelease)
        : null;
      if (!release) return;
      const asset = release.assets?.find((file) => /\.exe$/i.test(file.name));
      if (
        !asset ||
        !asset.browser_download_url.startsWith(
          "https://github.com/fphaikal/Nocan/releases/download/",
        )
      )
        return;
      $("#release-link").href = asset.browser_download_url;
      $("#release-link span").textContent = "Download for Windows";
      $("#release-note").textContent =
        `${release.tag_name} · Free under the MIT license. Windows desktop required.`;
    } catch {
      /* The source build remains available if GitHub is unavailable. */
    } finally {
      clearTimeout(timeout);
    }
  }

  $(".native-mascot").innerHTML = NocanPreview.mascot();
  renderHero();
  renderModule("music", false);
  renderStory("focus");
  setMotion(motionOff);
  loadSongLyrics();
  findRelease();
  setInterval(() => {
    const now = Date.now();
    if (
      state.musicPlaying &&
      !document.hidden &&
      (state.mode === "music" ||
        state.module === "music" ||
        state.scene === "music")
    ) {
      const elapsed = (now - lastMusicTick) / 1000;
      if (elapsed > 0) {
        state.musicPosition += elapsed;
        if (state.musicPosition >= NocanPreview.track.duration) {
          state.musicPosition = NocanPreview.track.duration;
          state.musicPlaying = false;
          refreshPreviews();
        }
        lastMusicTick = now;
        syncMusic();
      }
    } else lastMusicTick = now;
    if (state.running && !document.hidden) {
      const elapsed = Math.floor((now - lastTick) / 1000);
      if (elapsed >= 1) {
        state.remaining = Math.max(0, state.remaining - elapsed);
        lastTick += elapsed * 1000;
        if (state.remaining === 0) state.running = false;
        updateTimer();
      }
    } else lastTick = now;
  }, 250);
  window.addEventListener("load", () => {
    if (window.ScrollTrigger && !motionOff) ScrollTrigger.refresh();
  });
})();
