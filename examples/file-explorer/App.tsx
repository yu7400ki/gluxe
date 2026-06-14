import { fs } from "@gluxe/plugin-fs";
import type { DirEntry, Metadata } from "@gluxe/plugin-fs";
import { View, Text, Image, TextInput } from "gluxe";
import type { GpuiKeyboardEvent } from "gluxe";
import { useState, useEffect } from "react";

import fileIcon from "./src/icons/file.svg";
import folderIcon from "./src/icons/folder.svg";

// ---------------------------------------------------------------------------
// Path helpers (used to build candidate paths; always canonicalised by Rust)
// ---------------------------------------------------------------------------

/** Join a directory path and an entry name into a new path. */
function joinPath(dir: string, name: string): string {
  if (dir === ".") return name;
  return dir + "/" + name;
}

/**
 * Return the parent of `dir` by appending a `..` segment.  The caller is
 * expected to canonicalise the result via `fs.canonicalize` — Rust's
 * `canonicalize` resolves `..` and stops at the filesystem root, which is
 * why we don't need special-casing here.
 */
function parentOf(dir: string): string {
  return dir + "/..";
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

/** Format a byte count as a human-readable string (B / KB / MB / GB). */
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/**
 * Format an epoch-ms timestamp for display, or return "—" if null.
 *
 * NOTE: Boa's `Intl` currently resolves its default time zone to UTC (it does
 * not detect the host zone), so this renders the timestamp in UTC rather than
 * local time.  Pass an explicit `{ timeZone }` option if local time matters.
 */
function formatDate(ms: number | null): string {
  if (ms === null) return "—";
  return new Date(ms).toLocaleString();
}

// ---------------------------------------------------------------------------
// Colour palette
// ---------------------------------------------------------------------------

const C = {
  bg: "#1e1e2e", // window background
  headerBg: "#313244", // header bar
  paneBg: "#181825", // left / right pane background
  paneBorder: "#313244", // pane border / divider
  rowHover: "#313244", // entry row hover
  rowSelected: "#45475a", // selected file row
  rowCursorBorder: "#89b4fa", // keyboard-cursor row border (blue ring)
  dirColor: "#89b4fa", // directory name text (blue)
  fileColor: "#cdd6f4", // file name text (default text)
  headerText: "#cdd6f4", // header path text
  upBtn: "#313244", // up-button background
  upBtnHover: "#45475a",
  upBtnActive: "#585b70",
  upBtnText: "#cdd6f4",
  previewText: "#cdd6f4",
  placeholderText: "#585b70",
  errorText: "#f38ba8",
  statusBarBg: "#313244", // status bar background
  statusBarText: "#a6adc8", // status bar text (dimmer than normal)
};

// ---------------------------------------------------------------------------
// Image-type detection
// ---------------------------------------------------------------------------

const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "svg"]);

/** Return true if `path` has a known image file extension. */
function isImagePath(path: string): boolean {
  const dot = path.lastIndexOf(".");
  if (dot === -1) return false;
  return IMAGE_EXTS.has(path.slice(dot + 1).toLowerCase());
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

interface EntryRowProps {
  entry: DirEntry;
  path: string; // full relative path  (cwd + "/" + name)
  isSelected: boolean;
  /** True when the keyboard cursor is on this row (distinct from file selection). */
  isCursor: boolean;
  onClickDir: () => void;
  onClickFile: () => void;
}

function EntryRow({ entry, isSelected, isCursor, onClickDir, onClickFile }: EntryRowProps) {
  const label = entry.isDir ? entry.name + "/" : entry.name;
  const textColor = entry.isDir ? C.dirColor : C.fileColor;
  const baseBg = isSelected ? C.rowSelected : "transparent";

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "row",
        padding: 5, // 5px padding + 1px border = 6px visual height (same as before)
        borderRadius: 4,
        backgroundColor: baseBg,
        cursor: "pointer",
        // Show a coloured border ring when the keyboard cursor is on this row.
        // A 1px transparent border on non-cursor rows keeps layout stable.
        borderWidth: 1,
        borderColor: isCursor ? C.rowCursorBorder : "transparent",
        _hover: { backgroundColor: isSelected ? C.rowSelected : C.rowHover },
      }}
      onClick={entry.isDir ? onClickDir : onClickFile}
    >
      {/* file-type icon — folder or file SVG, 16×16 */}
      <Image src={entry.isDir ? folderIcon : fileIcon} style={{ width: 16, height: 16 }} />
      <Text
        style={{
          color: textColor,
          fontSize: 13,
          fontWeight: entry.isDir ? "bold" : "normal",
          whiteSpace: "nowrap",
          textOverflow: "ellipsis",
        }}
      >
        {label}
      </Text>
    </View>
  );
}

// ---------------------------------------------------------------------------
// Root component
// ---------------------------------------------------------------------------

export default function FileExplorer() {
  // Start with "." so the first readDir works immediately; the useEffect on
  // mount will replace this with the real absolute cwd from fs.cwd().
  const [cwd, setCwd] = useState<string>(".");
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [filter, setFilter] = useState<string>("");
  const [filterFocused, setFilterFocused] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [previewImage, setPreviewImage] = useState<string | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [selectedMeta, setSelectedMeta] = useState<Metadata | null>(null);
  const [metaError, setMetaError] = useState<string | null>(null);
  /**
   * Index of the keyboard-highlighted row within `visibleEntries`.
   * Moves with ↑/↓; Enter activates the highlighted entry.
   * Does NOT automatically open/preview files — only Enter does.
   */
  const [cursor, setCursor] = useState<number>(0);

  // On mount: switch the initial path to the real absolute cwd so the header
  // shows a meaningful absolute path from the very first render.
  useEffect(() => {
    fs.cwd()
      .then(setCwd)
      .catch(() => {
        /* keep "." if unavailable */
      });
  }, []);

  // Load directory entries whenever cwd changes.
  useEffect(() => {
    let cancelled = false;

    async function loadDir() {
      try {
        const raw = await fs.readDir(cwd);
        if (cancelled) return;

        // Sort: directories first, then files; within each group, alphabetically.
        const sorted = [...raw].sort((a, b) => {
          if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
          return a.name.localeCompare(b.name);
        });

        setEntries(sorted);
        setListError(null);
        // Clear selection / preview / metadata when navigating to a new directory.
        setSelected(null);
        setPreview(null);
        setPreviewImage(null);
        setPreviewError(null);
        setSelectedMeta(null);
        setMetaError(null);
        // Reset the keyboard cursor to the top of the new listing.
        setCursor(0);
      } catch (err) {
        if (!cancelled) {
          setListError(String(err));
        }
      }
    }

    loadDir();
    return () => {
      cancelled = true;
    };
  }, [cwd]);

  // Open a file: show an image preview or read its text content into the preview
  // pane, then fetch metadata in both cases.
  async function openFile(path: string) {
    setSelected(path);
    setPreview(null);
    setPreviewImage(null);
    setPreviewError(null);
    setSelectedMeta(null);
    setMetaError(null);

    if (isImagePath(path)) {
      // Render the image directly — don't try to read binary data as text.
      setPreviewImage(path);
    } else {
      try {
        const textResult = await fs.readTextFile(path);
        setPreview(textResult);
      } catch (err) {
        setPreviewError(`Cannot preview "${path}": ${String(err)}`);
      }
    }
    fs.metadata(path)
      .then(setSelectedMeta)
      .catch((err) => setMetaError(String(err)));
  }

  // Navigate into a directory: canonicalise to get an absolute path, then
  // update cwd.  Canonicalisation also handles ".." correctly and stops at
  // the filesystem root so we never accumulate stray ".." segments.
  async function navigateTo(rawPath: string) {
    try {
      const canonical = await fs.canonicalize(rawPath);
      setCwd(canonical);
    } catch {
      // Fallback: use the raw path (e.g. if the target doesn't exist yet).
      setCwd(rawPath);
    }
    setFilter("");
  }

  // Live-filtered entry list (case-insensitive substring match on name).
  const visibleEntries =
    filter.trim() === ""
      ? entries
      : entries.filter((e) => e.name.toLowerCase().includes(filter.toLowerCase()));

  // Clamp the cursor so it is always a valid index (or 0 when the list is empty).
  const clampedCursor =
    visibleEntries.length === 0 ? 0 : Math.min(cursor, visibleEntries.length - 1);

  /**
   * Keyboard handler for the entry-list container.
   *
   * Key map:
   *   ↓ / ↑          — move cursor down / up
   *   Home / End      — jump to first / last entry
   *   Page Down / Up  — move ±10 entries (clamped)
   *   Enter           — open file (preview) or enter directory
   *   Backspace       — go up one directory
   *   Escape          — clear the filter string (if any)
   */
  function onListKeyDown(e: GpuiKeyboardEvent) {
    const count = visibleEntries.length;
    switch (e.key) {
      case "down":
        setCursor((c) => Math.min(c + 1, Math.max(count - 1, 0)));
        break;
      case "up":
        setCursor((c) => Math.max(c - 1, 0));
        break;
      case "home":
        setCursor(0);
        break;
      case "end":
        setCursor(Math.max(count - 1, 0));
        break;
      case "pagedown":
        setCursor((c) => Math.min(c + 10, Math.max(count - 1, 0)));
        break;
      case "pageup":
        setCursor((c) => Math.max(c - 10, 0));
        break;
      case "enter": {
        const entry = visibleEntries[clampedCursor];
        if (!entry) break;
        const path = joinPath(cwd, entry.name);
        if (entry.isDir) {
          navigateTo(path);
        } else {
          openFile(path);
        }
        break;
      }
      case "backspace":
        navigateTo(parentOf(cwd));
        break;
      case "escape":
        if (filter) setFilter("");
        break;
      default:
        break;
    }
  }

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: C.bg,
        borderRadius: 6,
      }}
    >
      {/* Header bar */}
      <View
        style={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          gap: 10,
          padding: 10,
          backgroundColor: C.headerBg,
        }}
      >
        {/* Up button */}
        <View
          style={{
            backgroundColor: C.upBtn,
            padding: 6,
            borderRadius: 4,
            cursor: "pointer",
            _hover: { backgroundColor: C.upBtnHover },
            _active: { backgroundColor: C.upBtnActive },
          }}
          onClick={() => navigateTo(parentOf(cwd))}
        >
          <Text style={{ color: C.upBtnText, fontSize: 13 }}>↑ Up</Text>
        </View>

        {/* Current path */}
        <Text style={{ color: C.headerText, fontSize: 13 }}>{cwd}</Text>
      </View>

      {/* Body: two panes side-by-side */}
      <View
        style={{
          display: "flex",
          flexDirection: "row",
          flex: 1,
          gap: 1,
          overflowY: "hidden",
        }}
      >
        {/* Left pane — directory listing */}
        <View
          style={{
            display: "flex",
            flexDirection: "column",
            width: 300,
            backgroundColor: C.paneBg,
            borderWidth: 1,
            borderColor: C.paneBorder,
            borderRadius: 4,
            gap: 0,
          }}
        >
          {/* Filter bar */}
          <TextInput
            value={filter}
            placeholder="Filter…"
            onChangeText={setFilter}
            onFocus={() => setFilterFocused(true)}
            // Focus/blur events also carry the input's text in e.value.
            onBlur={() => setFilterFocused(false)}
            style={{
              padding: 6,
              backgroundColor: C.headerBg,
              color: C.fileColor,
              fontSize: 13,
              borderColor: filterFocused ? C.dirColor : C.paneBorder,
              borderWidth: 1,
            }}
          />

          {/* Entry list — autoFocus so keyboard nav works immediately on launch.
               Clicking the list (or any row) re-focuses it; clicking the filter
               input focuses that instead (focus follows the click target). */}
          <View
            autoFocus
            onKeyDown={onListKeyDown}
            style={{
              display: "flex",
              flexDirection: "column",
              flex: 1,
              padding: 8,
              gap: 2,
              overflowY: "scroll",
            }}
          >
            {listError !== null ? (
              <Text style={{ color: C.errorText, fontSize: 12 }}>{listError}</Text>
            ) : visibleEntries.length === 0 ? (
              <Text style={{ color: C.placeholderText, fontSize: 12 }}>
                {entries.length === 0 ? "(empty directory)" : "No matches."}
              </Text>
            ) : (
              visibleEntries.map((entry, index) => {
                const path = joinPath(cwd, entry.name);
                return (
                  <EntryRow
                    key={path}
                    entry={entry}
                    path={path}
                    isSelected={selected === path}
                    isCursor={index === clampedCursor}
                    onClickDir={() => navigateTo(path)}
                    onClickFile={() => openFile(path)}
                  />
                );
              })
            )}
          </View>
        </View>

        {/* Right pane — text preview */}
        <View
          style={{
            display: "flex",
            flexDirection: "column",
            flex: 1,
            padding: 12,
            backgroundColor: C.paneBg,
            overflowY: "scroll",
          }}
        >
          {previewError !== null ? (
            <Text style={{ color: C.errorText, fontSize: 12 }}>{previewError}</Text>
          ) : previewImage !== null ? (
            <Image src={previewImage} style={{ width: "100%", height: "100%", borderRadius: 4 }} />
          ) : preview !== null ? (
            <Text style={{ color: C.previewText, fontSize: 12 }}>{preview}</Text>
          ) : selected !== null ? (
            <Text style={{ color: C.placeholderText, fontSize: 12 }}>Loading…</Text>
          ) : (
            <Text style={{ color: C.placeholderText, fontSize: 12 }}>
              Select a file to preview its contents.
            </Text>
          )}
        </View>
      </View>

      {/* Status bar — shows size and modified date for the selected file */}
      <View
        style={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          gap: 16,
          padding: 6,
          paddingLeft: 12,
          backgroundColor: C.statusBarBg,
        }}
      >
        {selectedMeta !== null ? (
          <>
            <Text style={{ color: C.statusBarText, fontSize: 11 }}>
              {formatBytes(selectedMeta.size)}
            </Text>
            <Text style={{ color: C.statusBarText, fontSize: 11 }}>
              Modified: {formatDate(selectedMeta.modified)}
            </Text>
          </>
        ) : metaError !== null ? (
          <Text style={{ color: C.errorText, fontSize: 11 }}>{metaError}</Text>
        ) : (
          <Text style={{ color: C.placeholderText, fontSize: 11 }}>
            {selected !== null ? "Loading metadata…" : ""}
          </Text>
        )}
      </View>
    </View>
  );
}
