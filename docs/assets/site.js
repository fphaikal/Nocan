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
    mode: "focus",
    module: "focus",
    remaining: 1500,
    running: false,
    started: false,
    musicPlaying: true,
    water: 500,
    taskDone: [false, false, false],
    open: false,
    scene: "focus",
    theme: "light",
  };
  let motionOff = reducedMotion.matches;
  let motionContext;
  let toastTimeout;
  let lastTick = Date.now();

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

  function focusMarkup(illustrative = false) {
    return `<div class="mini-focus"><div class="mini-ring">${icon("focus")}</div><div><div class="mini-time" ${illustrative ? "" : "data-timer"}>${illustrative ? "24:59" : formatTime(state.remaining)}</div><span class="mini-timer-description">${illustrative ? "A little time for your next idea" : state.running ? "One thing at a time" : "Ready when you are"}</span></div><div class="mini-focus-actions">${illustrative ? '<span class="mini-action">Focus session</span>' : `<button type="button" class="mini-action" data-timer-action>${icon(state.running ? "pause" : "play")}<span>${state.running ? "Pause" : state.started ? "Resume" : "Start"}</span></button><button type="button" class="mini-quiet" data-final-minute>Try final minute</button>`}</div></div>`;
  }

  function musicMarkup(illustrative = false) {
    return `<div class="mini-music${state.musicPlaying ? "" : " is-paused"}"><div class="album-art">${icon("music")}</div><div class="mini-track"><strong>Quiet hours</strong><span>Browser preview · no audio</span></div>${illustrative ? '<div class="equalizer"><i></i><i></i><i></i><i></i></div>' : `<button type="button" class="music-control" data-music-action aria-label="${state.musicPlaying ? "Pause" : "Play"} music preview">${icon(state.musicPlaying ? "pause" : "play")}</button>`}</div><div class="mini-lyric">${state.musicPlaying || illustrative ? "A little room to find your rhythm." : "A little pause in the music."}</div>`;
  }

  function waterMarkup(illustrative = false) {
    const water = illustrative ? 750 : state.water;
    return `<div class="mini-water"><div class="water-glass"><div class="water-fill" style="--water-level:${Math.min((water / 2000) * 100, 100)}%"></div>${icon("water")}</div><div class="water-stat"><span ${illustrative ? "" : "data-water"} style="font:inherit;color:inherit">${(water / 1000).toFixed(2).replace(/0$/, "")}</span> <span>/ 2 L</span><small>Today · a little closer</small></div><div class="water-actions">${illustrative ? '<span class="mini-action">Water logged!</span>' : '<button class="mini-action" type="button" data-drink>+250 ml</button>'}</div></div>`;
  }

  function tasksMarkup() {
    const tasks = [
      "Make room for a good idea",
      "Take a little break",
      "Finish something that matters",
    ];
    return `<div class="task-done-count">${state.taskDone.filter(Boolean).length} / 3 done</div><div class="task-preview">${tasks.map((task, index) => `<button class="task-check" type="button" data-task="${index}" aria-pressed="${state.taskDone[index]}">${task}</button>`).join("")}</div>`;
  }

  function sampleMarkup(module) {
    if (module === "focus") return focusMarkup();
    if (module === "music") return musicMarkup();
    if (module === "water") return waterMarkup();
    if (module === "tasks") return tasksMarkup();
    if (module === "files")
      return `<div class="sample-row">${icon("files")}<div><strong>A good idea.pdf</strong><small>Ready when you need it</small></div><span class="sample-value">PDF</span></div><div class="sample-row">${icon("files")}<div><strong>Weekend plans.txt</strong><small>A little something for later</small></div><span class="sample-value">TXT</span></div>`;
    if (module === "openmaus")
      return `<div class="sample-row">${icon("check")}<div><strong>Agent finished responding</strong><small>Research assistant · example activity</small></div><span class="sample-value">Done</span></div><div class="sample-row">${icon("bot")}<div><strong>Daily summary</strong><small>Example automation schedule</small></div><span class="sample-value">09:00</span></div>`;
    if (module === "usage")
      return '<div class="sample-usage"><div><span>Example daily tokens</span><strong>24.8k</strong><small>Local usage records</small></div><div><span>Example active model</span><strong style="font-size:22px">Your model</strong><small>Input + output</small></div></div><svg class="sample-chart" viewBox="0 0 300 45" aria-hidden="true"><path d="M0 35h28l9-8 18 3 15-11 22 5 10-15 15 5 25-3 10 5 25-10 18 5 10-9 20 8 20-6 15 8 15-4 25 3"/></svg>';
    return '<div class="sample-price">AAPL <small>+1.24%</small></div><div class="sample-tag">Illustrative ticker · sample data</div><svg class="sample-chart" viewBox="0 0 300 45" aria-hidden="true"><path d="m0 40 17-5 11 3 20-11 15 5 25-15 21 6 18-10 16 8 23-17 15 7 23-5 15 4 20-7 18 5 23-7"/></svg>';
  }

  function formatTime(seconds) {
    return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
  }

  function timerColor() {
    if (state.remaining === 0) return "#245bb5";
    if (!state.running) return "#4e5563";
    if (state.remaining <= 60) return "#a5363f";
    if (state.remaining <= 375) return "#80531b";
    return "#154f3d";
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
    const detached =
      state.mode === "music" && state.musicPlaying && state.started;
    $(".floating-timer").hidden = !detached;
    $(".floating-timer").style.backgroundColor = timerColor();
    $(".compact-timer").style.backgroundColor = timerColor();
    $(".compact-timer").hidden = !state.started || state.open || detached;
    $(".tiger-eyes").style.display =
      state.started && !state.open && !detached ? "none" : "";
  }

  function renderHero() {
    const data = modules[state.mode];
    $(".notch-demo").dataset.demoMode = state.mode;
    $(".mini-section-label").innerHTML =
      icon(data.icon) + `<span>${data.title}</span>`;
    $(".hero-panel-content").innerHTML = sampleMarkup(state.mode);
    $$("[data-hero-mode]").forEach((button) =>
      button.setAttribute(
        "aria-pressed",
        String(button.dataset.heroMode === state.mode),
      ),
    );
    updateTimer();
  }

  function openNotch(open = true) {
    state.open = open;
    $(".notch-demo").classList.toggle("is-open", open);
    $(".notch-face").setAttribute("aria-expanded", String(open));
    $(".notch-face").setAttribute(
      "aria-label",
      `${open ? "Collapse" : "Expand"} Nocan interactive preview`,
    );
    $(".hero-demo-panel").inert = !open;
    updateTimer();
  }

  function renderModule(module, animate = true) {
    state.module = module;
    const data = modules[module];
    $$("[data-module]").forEach((button) => {
      const active = button.dataset.module === module;
      button.setAttribute("aria-selected", String(active));
      button.tabIndex = active ? 0 : -1;
    });
    const panel = $(".module-panel");
    panel.setAttribute("aria-labelledby", `module-${module}`);
    $(".explorer-preview").innerHTML =
      `<div class="mini-header"><span class="mini-section-label">${icon(data.icon)}<span>${data.title}</span></span><span style="color:#a5adbb;font-size:9px">Nocan preview</span></div>${sampleMarkup(module)}`;
    $(".module-kicker").textContent = data.kicker;
    $(".module-description h3").textContent = data.heading;
    $(".module-description p").textContent = data.description;
    if (animate && !motionOff && window.gsap)
      gsap.fromTo(
        $(".explorer-preview"),
        { opacity: 0.3, y: 8 },
        { opacity: 1, y: 0, duration: 0.35, overwrite: true },
      );
    updateTimer();
  }

  function renderStory(scene) {
    if (!modules[scene]) return;
    state.scene = scene;
    const data = modules[scene];
    const panel = $(".story-notch");
    panel.innerHTML = `<div class="mini-header"><span class="mini-section-label">${icon(data.icon)}<span>${data.title}</span></span><span style="color:#9da9ba;font-size:9px">9:41</span></div>${scene === "focus" ? focusMarkup(true) : scene === "music" ? musicMarkup(true) : waterMarkup(true)}`;
    $(".flow-stage").dataset.scene = scene;
    const index = ["focus", "music", "water"].indexOf(scene);
    $(".scene-label").textContent = [
      "A little focus",
      "A little soundtrack",
      "A little care",
    ][index];
    $(".scene-count").textContent = `0${index + 1} / 03`;
    $(".scene-track span").style.transform = `translateX(${index * 100}%)`;
    if (window.gsap && !motionOff) {
      gsap.fromTo(
        panel,
        { y: 12, opacity: 0.4 },
        { y: 0, opacity: 1, duration: 0.55, overwrite: true },
      );
      gsap.to($(".story-chip"), {
        autoAlpha: scene === "music" ? 1 : 0,
        x: scene === "music" ? 0 : -25,
        y: scene === "music" ? 0 : 15,
        duration: 0.5,
        ease: "power3.out",
        overwrite: true,
      });
    } else {
      $(".story-chip").style.opacity = scene === "music" ? "1" : "0";
      $(".story-chip").style.visibility =
        scene === "music" ? "visible" : "hidden";
      $(".story-chip").style.transform = "none";
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
    clearTimeout(toastTimeout);
    toastTimeout = setTimeout(
      () => confirmation.classList.remove("is-visible"),
      2200,
    );
    if (window.gsap && !motionOff)
      gsap.fromTo(
        $$(".water-glass"),
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
          gsap.to(".wallpaper", {
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
          ".story-notch, .story-chip, .pupils, .water-glass, .explorer-preview",
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
  $(".mini-close").addEventListener("click", () => {
    openNotch(false);
    $(".notch-face").focus({ preventScroll: true });
  });
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
      openNotch(false);
      if ($(".notch-demo").contains(document.activeElement))
        $(".notch-face").focus({ preventScroll: true });
    }
  });

  document.addEventListener("click", (event) => {
    const focusHost = event.target.closest(
      ".hero-panel-content, .explorer-preview, .floating-timer",
    );
    const activeControl = [
      "data-timer-action",
      "data-final-minute",
      "data-music-action",
    ].find((attribute) => event.target.closest(`[${attribute}]`));
    if (event.target.closest("[data-timer-action]")) {
      if (state.remaining === 0) state.remaining = 1500;
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
      state.water = Math.min(state.water + 250, 3000);
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
      renderModule("tasks", false);
      $(`[data-task="${index}"]`).focus({ preventScroll: true });
    }
    if (focusHost && activeControl)
      $(`[${activeControl}]`, focusHost)?.focus({ preventScroll: true });
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

  renderHero();
  renderModule("focus", false);
  renderStory("focus");
  setMotion(motionOff);
  findRelease();
  setInterval(() => {
    const now = Date.now();
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
