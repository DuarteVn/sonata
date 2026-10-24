import { GitBranch } from "lucide-react";
import type { Lane, LaneState, LaneStatus } from "../ipc/types";

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

const BROKEN: Partial<Record<LaneState, string>> = {
  checkoutMissing: "folder missing",
  branchMissing: "branch missing",
  mainCheckoutMissing: "repository missing",
  gitUnavailable: "git unavailable",
  busy: "merge in progress",
};

// The lane's branch and state on a terminal node (feature 008, FR-018).
export function LaneBadge({ lane, status }: { lane: Lane; status?: LaneStatus }) {
  const broken = status ? BROKEN[status.state] : undefined;
  const changed = status?.changed ?? 0;
  const ahead = status?.ahead ?? 0;
  const behind = status?.behind ?? 0;
  return (
    <span className={`lane-badge${broken ? " broken" : ""}`} title={`Lane ${lane.branch} from ${lane.baseBranch}`}>
      <GitBranch size={11} aria-hidden />
      <span className="lane-branch">{lane.branch}</span>
      {changed > 0 && <span aria-label={plural(changed, "uncommitted file")}>●{changed}</span>}
      {ahead > 0 && <span aria-label={`${plural(ahead, "commit")} ahead of ${lane.baseBranch}`}>↑{ahead}</span>}
      {behind > 0 && <span aria-label={`${plural(behind, "commit")} behind ${lane.baseBranch}`}>↓{behind}</span>}
      {broken && <span className="lane-broken">{broken}</span>}
    </span>
  );
}
