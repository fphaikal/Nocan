(() => {
  "use strict";

  const excerpt = "Merry Christmas, please don’t call";
  const duration = 202;
  const defaultCues = () => [
    { time: 0, text: "" },
    { time: 57.36, text: excerpt },
    { time: 65.19, text: "" },
    { time: 117.18, text: excerpt },
    { time: 124.95, text: "" },
    { time: 160.15, text: excerpt },
    { time: 164.44, text: "" },
  ];

  function at(cues, position) {
    let low = 0,
      high = cues.length - 1,
      index = -1;
    while (low <= high) {
      const mid = (low + high) >> 1;
      if (cues[mid].time <= position) {
        index = mid;
        low = mid + 1;
      } else high = mid - 1;
    }
    const cue = cues[index];
    const end = cues[index + 1]?.time ?? duration;
    return {
      index,
      text: cue?.text || "",
      start: cue?.time ?? 0,
      end,
      progress: cue
        ? Math.max(
            0,
            Math.min(1, (position - cue.time) / Math.max(0.01, end - cue.time)),
          )
        : 0,
    };
  }

  function parse(text) {
    const offset =
      Number(text.match(/\[offset:\s*([+-]?\d+)\]/i)?.[1] || 0) / 1000;
    const entries = new Map();
    for (const line of text.replace(/^\uFEFF/, "").split(/\r?\n/)) {
      const stamps = [
        ...line.matchAll(/\[(\d{1,3}):([0-5]\d)(?:[.:](\d{1,3}))?\]/g),
      ];
      if (!stamps.length) continue;
      const caption = line.replace(/\[[^\]]*\]/g, "").trim();
      for (const stamp of stamps) {
        const time = Math.max(
          0,
          Number(stamp[1]) * 60 +
            Number(stamp[2]) +
            Number(`0.${stamp[3] || 0}`) +
            offset,
        );
        if (time < duration) entries.set(time, caption);
      }
    }
    const cues = [...entries]
      .sort((a, b) => a[0] - b[0])
      .map(([time, text]) => ({ time, text }));
    if (!cues.some((cue) => cue.text))
      throw new Error(
        "No timed lyrics found for this song. Choose a timestamped .lrc file.",
      );
    if (cues[0].time > 0) cues.unshift({ time: 0, text: "" });
    return cues;
  }

  const normalize = (value) =>
    String(value || "")
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]/gu, "");

  async function load(track, { signal, fetchImpl = fetch } = {}) {
    const params = new URLSearchParams({
      artist_name: track.artist,
      track_name: track.title.replace(/’/g, "'"),
      duration: String(track.duration),
    });
    const request = async (path) => {
      const query = new URLSearchParams(params);
      if (path === "search") query.delete("duration");
      const response = await fetchImpl(
        `https://lrclib.net/api/${path}?${query}`,
        {
          signal,
          headers: { Accept: "application/json" },
          credentials: "omit",
          referrerPolicy: "no-referrer",
        },
      );
      if (response.status === 404) return null;
      if (!response.ok)
        throw new Error(
          "Lyrics service is unavailable. Try again in a moment.",
        );
      return response.json();
    };
    const matches = (item) =>
      item &&
      normalize(item.artistName) === normalize(track.artist) &&
      normalize(item.trackName) === normalize(track.title) &&
      Math.abs(Number(item.duration) - track.duration) <= 2 &&
      typeof item.syncedLyrics === "string" &&
      item.syncedLyrics.trim();
    let record = await request("get");
    if (!matches(record)) {
      const results = await request("search");
      record = Array.isArray(results) ? results.find(matches) : null;
    }
    if (!matches(record))
      throw new Error(
        "Time-synced lyrics are unavailable for this recording. Retry or load a local LRC.",
      );
    return {
      cues: parse(record.syncedLyrics),
      id: record.id,
      source: "LRCLIB",
    };
  }

  window.NocanLyrics = { excerpt, duration, defaultCues, at, parse, load };
})();
