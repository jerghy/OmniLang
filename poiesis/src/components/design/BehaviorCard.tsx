// poiesis/src/components/design/BehaviorCard.tsx
// M3 T1 行为卡：当[事件]→[动作][参数] 表单。意图层——事件用口语名（click 等），
// T2 映射到语义事件（onPress）并接线 applyBehaviorCard。
import { useMemo, useState } from "react";
import { ACTIONS } from "./action-verbs";
import { useSignalItems } from "../panels/inspector/shared";
import styles from "./BehaviorCard.module.css";

/** 意图层事件选项（口语名）；T2 映射到语义事件（click→onPress 等）。 */
const EVENT_OPTIONS: Array<{ value: string; label: string }> = [
  { value: "click", label: "点击" },
  { value: "submit", label: "提交" },
  { value: "change", label: "变更" },
  { value: "focus", label: "聚焦" },
  { value: "blur", label: "失焦" },
  { value: "keydown", label: "按键" },
];

export interface BehaviorCardCommit {
  elementId: string;
  event: string;
  action: string;
  params: Record<string, string>;
}

interface BehaviorCardProps {
  elementId: string;
  onCommit: (behavior: BehaviorCardCommit) => void;
}

export function BehaviorCard({ elementId, onCommit }: BehaviorCardProps) {
  const signalItems = useSignalItems();

  const [event, setEvent] = useState("");
  const [action, setAction] = useState("");
  const [params, setParams] = useState<Record<string, string>>({});

  const actionDef = action ? ACTIONS[action] : undefined;

  const signalOptions = useMemo(() => {
    const options = signalItems.map((item) => ({ value: item.name, label: item.name }));
    // 去重（同名字段多来源时保第一个）
    const seen = new Set<string>();
    return options.filter((o) => {
      if (seen.has(o.value)) return false;
      seen.add(o.value);
      return true;
    });
  }, [signalItems]);

  const handleActionChange = (value: string) => {
    setAction(value);
    setParams({});
  };

  const handleParamChange = (key: string, value: string) => {
    setParams((prev) => ({ ...prev, [key]: value }));
  };

  const handleCommit = () => {
    onCommit({ elementId, event, action, params });
  };

  return (
    <div data-testid="behavior-card" className={styles.card}>
      <span className={styles.whenLabel}>当</span>
      <select
        data-testid="behavior-event"
        className={styles.select}
        value={event}
        onChange={(e) => setEvent(e.target.value)}
        aria-label="触发事件"
      >
        <option value="">选择事件</option>
        {EVENT_OPTIONS.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>

      <span className={styles.arrow}>→</span>

      <select
        data-testid="behavior-action"
        className={styles.select}
        value={action}
        onChange={(e) => handleActionChange(e.target.value)}
        aria-label="行为动作"
      >
        <option value="">选择动作</option>
        {Object.entries(ACTIONS).map(([key, def]) => (
          <option key={key} value={key}>
            {def.label}
          </option>
        ))}
      </select>

      {actionDef &&
        actionDef.params.map((param, index) => {
          const testId = index === 0 ? "behavior-target" : undefined;
          if (param.kind === "signal") {
            return (
              <select
                key={param.key}
                data-testid={testId}
                className={styles.select}
                value={params[param.key] ?? ""}
                onChange={(e) => handleParamChange(param.key, e.target.value)}
                aria-label={param.label}
              >
                <option value="">{param.label}</option>
                {signalOptions.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            );
          }
          if (param.kind === "page") {
            return (
              <input
                key={param.key}
                data-testid={testId}
                className={styles.input}
                placeholder={param.label}
                value={params[param.key] ?? ""}
                onChange={(e) => handleParamChange(param.key, e.target.value)}
                aria-label={param.label}
              />
            );
          }
          // text / none：自由文本参数（步长等）
          return (
            <input
              key={param.key}
              data-testid={testId}
              className={styles.input}
              placeholder={param.label}
              value={params[param.key] ?? ""}
              onChange={(e) => handleParamChange(param.key, e.target.value)}
              aria-label={param.label}
            />
          );
        })}

      <button
        type="button"
        data-testid="behavior-commit"
        className={styles.commitBtn}
        onClick={handleCommit}
        disabled={!event || !action}
      >
        添加行为
      </button>
    </div>
  );
}
