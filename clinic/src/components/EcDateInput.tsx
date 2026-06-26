import { useState } from "react";
import type { EcDate } from "../lib/ethiopian";
import {
  ETHIOPIAN_MONTHS,
  ecMonthStartWeekday,
  ethiopianMonthLength,
  todayEc,
} from "../lib/ethiopian";

type Props = {
  value: EcDate | null;
  onChange: (value: EcDate) => void;
};

export function EcDateInput({ value, onChange }: Props) {
  const today = todayEc();
  const [navYear, setNavYear] = useState(value?.year ?? today.year - 30);
  const [navMonth, setNavMonth] = useState(value?.month ?? today.month);

  const prevMonth = () => {
    if (navMonth === 1) { setNavYear((y) => y - 1); setNavMonth(13); }
    else setNavMonth((m) => m - 1);
  };

  const nextMonth = () => {
    if (navMonth === 13) { setNavYear((y) => y + 1); setNavMonth(1); }
    else setNavMonth((m) => m + 1);
  };

  const maxDay = ethiopianMonthLength(navYear, navMonth);
  const startWeekday = ecMonthStartWeekday(navYear, navMonth);
  const cells: (number | null)[] = [
    ...Array<null>(startWeekday).fill(null),
    ...Array.from({ length: maxDay }, (_, i) => i + 1),
  ];

  const isSelected = (d: number) =>
    value !== null &&
    value.year === navYear &&
    value.month === navMonth &&
    value.day === d;

  return (
    <div className="ec-calendar">
      <div className="ec-cal-header">
        <button type="button" className="ghost" onClick={prevMonth}>←</button>
        <span className="ec-cal-month">{ETHIOPIAN_MONTHS[navMonth - 1]}</span>
        <input
          type="number"
          className="ec-cal-year"
          value={navYear}
          onChange={(e) => { const y = Number(e.target.value); if (y > 0) setNavYear(y); }}
          aria-label="Year (Ethiopian)"
        />
        <button type="button" className="ghost" onClick={nextMonth}>→</button>
      </div>
      <div className="ec-cal-label">Ethiopian calendar</div>
      <div className="ec-cal-grid">
        {["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"].map((d) => (
          <div key={d} className="ec-cal-weekday">{d}</div>
        ))}
        {cells.map((day, i) =>
          day === null ? (
            <div key={`empty-${i}`} />
          ) : (
            <button
              key={day}
              type="button"
              className={`ec-cal-day${isSelected(day) ? " ec-cal-day--selected" : ""}`}
              onClick={() => onChange({ year: navYear, month: navMonth, day })}
            >
              {day}
            </button>
          )
        )}
      </div>
    </div>
  );
}
