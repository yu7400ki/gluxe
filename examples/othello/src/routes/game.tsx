import { Text, View } from "@gluxe/react";
import { useLocation, useNavigate } from "@gluxe/router";
import { useCallback, useEffect, useRef, useState } from "react";

import { BoardView } from "../components/BoardView";
import { GameOver } from "../components/GameOver";
import { StartConfig } from "../components/Menu";
import { ScorePanel } from "../components/ScorePanel";
import { Toast } from "../components/Toast";
import { useAnimationFrame } from "../hooks/useAnimationFrame";
import { Cell, StateView, TurnColor, cpuMove, newGame, play, toDto } from "../othello";
import { BOARD_SIZE, C, FLIP_MS, FLIP_STAGGER_MS, chebyshev } from "../theme";

/** Total duration of a flip animation given which cells flipped from where. */
function animDuration(state: StateView): number {
  if (state.lastMove === null) return 0;
  let maxDelay = 0;
  for (const i of state.flipped) {
    maxDelay = Math.max(maxDelay, chebyshev(i, state.lastMove));
  }
  return maxDelay * FLIP_STAGGER_MS + FLIP_MS + 40;
}

const PASS_TOAST_MS = 1300;
const CPU_DELAY_MS = 420;

/** Game screen. Receives its `StartConfig` via `navigate("/game", { state })`. */
export default function GamePage() {
  const navigate = useNavigate();
  const location = useLocation();
  const config = (location.state as StartConfig | undefined) ?? null;

  // Guard: navigating here without a config (shouldn't happen) returns to the menu.
  useEffect(() => {
    if (!config) navigate("/", { replace: true });
  }, [config, navigate]);

  const [state, setState] = useState<StateView | null>(null);

  // Animation: prevCells holds the board face before the latest move so flipped
  // discs can show their old colour during the first half of the flip.
  const [prevCells, setPrevCells] = useState<Cell[] | null>(null);
  const [animT, setAnimT] = useState(0);
  const [animating, setAnimating] = useState(false);
  const animStartRef = useRef<number | null>(null);
  const animTotalRef = useRef(0);

  const [toast, setToast] = useState<string | null>(null);

  // CPU colour (the side the computer controls), or null in free mode.
  const cpuColor: TurnColor | null =
    config && config.mode === "cpu" ? (config.playerColor === "black" ? "white" : "black") : null;

  // Fetch a fresh board when the game starts (route mount).
  useEffect(() => {
    if (!config) return;
    let cancelled = false;
    newGame().then((fresh) => {
      if (cancelled) return;
      setPrevCells(null);
      setState(fresh);
    });
    return () => {
      cancelled = true;
    };
  }, [config]);

  // Drive the flip animation clock.
  useAnimationFrame(animating, (ts) => {
    if (animStartRef.current === null) animStartRef.current = ts;
    const t = ts - animStartRef.current;
    if (t >= animTotalRef.current) {
      setAnimating(false);
      setAnimT(0);
      animStartRef.current = null;
    } else {
      setAnimT(t);
    }
  });

  /** Commit a new state and kick off its flip animation + pass toast. */
  const commit = useCallback((prev: StateView | null, next: StateView) => {
    setState(next);
    if (next.lastMove !== null) {
      setPrevCells(prev ? prev.cells : null);
      const total = animDuration(next);
      if (total > 0) {
        animStartRef.current = null;
        animTotalRef.current = total;
        setAnimT(0);
        setAnimating(true);
      }
    }
    if (next.passed) {
      // The side that *just* moved forced the *other* side to pass; `turn` is
      // the player who still has to play after the pass.
      const passer: TurnColor = next.turn === "black" ? "white" : "black";
      setToast(passer === "black" ? "黒はパス" : "白はパス");
    }
  }, []);

  const backToMenu = useCallback(() => {
    navigate("/");
  }, [navigate]);

  const retry = useCallback(async () => {
    setAnimating(false);
    setAnimT(0);
    setToast(null);
    const fresh = await newGame();
    setPrevCells(null);
    setState(fresh);
  }, []);

  // Auto-dismiss the pass toast.
  useEffect(() => {
    if (toast === null) return;
    const t = setTimeout(() => setToast(null), PASS_TOAST_MS);
    return () => clearTimeout(t);
  }, [toast]);

  const inputLocked = animating;

  // Human move.
  const handleCellClick = useCallback(
    (index: number) => {
      if (!state || state.ended || inputLocked) return;
      // In CPU mode, ignore clicks during the CPU's turn.
      if (cpuColor && state.turn === cpuColor) return;
      const prev = state;
      play(toDto(state), index).then((next) => {
        if (next.lastMove === null) return; // rejected
        commit(prev, next);
      });
    },
    [state, inputLocked, cpuColor, commit],
  );

  // CPU driver: when it is the CPU's turn, not ended, and no animation is in
  // flight, think after a short delay and apply the move. Re-runs after every
  // state change so consecutive CPU turns (when the human passes) are handled.
  useEffect(() => {
    if (!state || !cpuColor || !config) return;
    if (state.ended || animating) return;
    if (state.turn !== cpuColor) return;

    let cancelled = false;
    const t = setTimeout(async () => {
      const prev = state;
      const result = await cpuMove(toDto(state), config.level);
      if (cancelled) return;
      commit(prev, result.state);
    }, CPU_DELAY_MS);

    return () => {
      cancelled = true;
      clearTimeout(t);
    };
  }, [state, cpuColor, config, animating, commit]);

  // Player / CPU labels.
  const labels = (() => {
    if (!config || config.mode === "free") {
      return { black: "黒", white: "白" };
    }
    const human = config.playerColor;
    return {
      black: human === "black" ? "あなた" : "CPU",
      white: human === "white" ? "あなた" : "CPU",
    };
  })();

  const levelLabel = (() => {
    if (!config || config.mode === "free") return "ひとりで打つ";
    const name =
      config.level === "easy" ? "やさしい" : config.level === "normal" ? "ふつう" : "つよい";
    return `CPU対戦 · ${name}`;
  })();

  // Animated thinking dots (cycles 1..3).
  const [dots, setDots] = useState(1);
  const thinking = !!(cpuColor && state && !state.ended && state.turn === cpuColor);
  useAnimationFrame(thinking, (ts) => {
    setDots((Math.floor(ts / 350) % 3) + 1);
  });

  // Game-over title.
  const gameOverTitle = (() => {
    if (!state) return "";
    const b = state.blackCount;
    const w = state.whiteCount;
    if (!config || config.mode === "free") {
      if (b === w) return "引き分け";
      return b > w ? "黒の勝ち" : "白の勝ち";
    }
    const humanCount = config.playerColor === "black" ? b : w;
    const cpuCount = config.playerColor === "black" ? w : b;
    if (humanCount === cpuCount) return "引き分け";
    return humanCount > cpuCount ? "あなたの勝ち！" : "CPUの勝ち";
  })();

  if (!config || !state) return null;

  return (
    <View
      style={{
        position: "relative",
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        paddingX: 28,
        paddingTop: 4,
        paddingBottom: 14,
        gap: 14,
      }}
    >
      {/* Score panel */}
      <ScorePanel
        turn={state.turn}
        blackCount={state.blackCount}
        whiteCount={state.whiteCount}
        blackName={labels.black}
        whiteName={labels.white}
        ended={state.ended}
        cpuColor={cpuColor}
        thinking={thinking}
        dots={dots}
      />

      {/* Board */}
      <View
        style={{
          display: "flex",
          flex: 1,
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <View style={{ position: "relative", width: BOARD_SIZE, height: BOARD_SIZE }}>
          <BoardView
            state={state}
            prevCells={prevCells}
            animT={animT}
            animating={animating}
            onCellClick={handleCellClick}
            hideHints={state.ended || (!!cpuColor && state.turn === cpuColor) || inputLocked}
          />
          {toast && !state.ended && <Toast message={toast} />}
          {state.ended && (
            <GameOver
              title={gameOverTitle}
              blackCount={state.blackCount}
              whiteCount={state.whiteCount}
              onRetry={retry}
              onMenu={backToMenu}
            />
          )}
        </View>
      </View>

      {/* Footer: mode label */}
      <View style={{ display: "flex", flexDirection: "row", justifyContent: "center" }}>
        <Text style={{ fontSize: 12, fontWeight: "bold", color: C.textSecondary }}>
          {levelLabel}
        </Text>
      </View>
    </View>
  );
}
