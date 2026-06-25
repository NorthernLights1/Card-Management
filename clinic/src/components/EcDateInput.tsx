import type { EcDate } from "../lib/ethiopian";
import { ETHIOPIAN_MONTHS, ethiopianMonthLength, todayEc } from "../lib/ethiopian";

type Props = {
  value: EcDate | null;
  onChange: (value: EcDate) => void;
};

// Date-of-birth entry directly in the Ethiopian calendar (year / month / day).
export function EcDateInput({ value, onChange }: Props) {
  const current = value ?? todayEc();
  const maxDay = ethiopianMonthLength(current.year, current.month);

  const set = (patch: Partial<EcDate>) => {
    const next = { ...current, ...patch };
    const clampDay = Math.min(next.day, ethiopianMonthLength(next.year, next.month));
    onChange({ ...next, day: clampDay });
  };

  return (
    <div className="ec-date">
      <input
        type="number"
        aria-label="Year (Ethiopian)"
        min={1}
        max={3000}
        value={current.year}
        onChange={(e) => set({ year: Number(e.target.value) })}
      />
      <select
        aria-label="Month (Ethiopian)"
        value={current.month}
        onChange={(e) => set({ month: Number(e.target.value) })}
      >
        {ETHIOPIAN_MONTHS.map((name, i) => (
          <option key={name} value={i + 1}>
            {i + 1} · {name}
          </option>
        ))}
      </select>
      <input
        type="number"
        aria-label="Day (Ethiopian)"
        min={1}
        max={maxDay}
        value={current.day}
        onChange={(e) => set({ day: Number(e.target.value) })}
      />
    </div>
  );
}
