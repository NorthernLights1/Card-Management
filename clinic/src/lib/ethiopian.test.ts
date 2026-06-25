import { describe, it, expect } from "vitest";
import {
  ecToGregorian,
  gregorianToEc,
  ageFromEcDob,
  ageFromRecorded,
  isEthiopianLeapYear,
  ethiopianMonthLength,
} from "./ethiopian";

describe("EC <-> Gregorian conversion", () => {
  // Anchors: 1 Meskerem (Ethiopian New Year). Falls on Sep 12 in the Gregorian
  // year before a leap year, otherwise Sep 11.
  const anchors: Array<[EcTuple, GcTuple]> = [
    [[2000, 1, 1], [2007, 9, 12]],
    [[2012, 1, 1], [2019, 9, 12]],
    [[2016, 1, 1], [2023, 9, 12]],
    [[2017, 1, 1], [2024, 9, 11]],
  ];

  it.each(anchors)("EC %j -> GC %j", (ec, gc) => {
    const g = ecToGregorian({ year: ec[0], month: ec[1], day: ec[2] });
    expect([g.year, g.month, g.day]).toEqual(gc);
  });

  it.each(anchors)("GC %j -> EC %j (round trip)", (ec, gc) => {
    const e = gregorianToEc(gc[0], gc[1], gc[2]);
    expect([e.year, e.month, e.day]).toEqual(ec);
  });
});

describe("Ethiopian leap year + Pagume length", () => {
  it("year % 4 === 3 is a leap year", () => {
    expect(isEthiopianLeapYear(2011)).toBe(true); // 2011 % 4 === 3
    expect(isEthiopianLeapYear(2012)).toBe(false);
  });
  it("Pagume has 6 days in a leap year, else 5", () => {
    expect(ethiopianMonthLength(2011, 13)).toBe(6);
    expect(ethiopianMonthLength(2012, 13)).toBe(5);
    expect(ethiopianMonthLength(2012, 1)).toBe(30);
  });
});

describe("age computation", () => {
  it("computes age from an Ethiopian DOB", () => {
    // Born 1 Meskerem 2000 EC (= 2007-09-12 GC). As of 2024-09-12 GC -> 17.
    const age = ageFromEcDob({ year: 2000, month: 1, day: 1 }, new Date(2024, 8, 12));
    expect(age).toBe(17);
    // One day before the birthday -> still 16.
    const before = ageFromEcDob({ year: 2000, month: 1, day: 1 }, new Date(2024, 8, 11));
    expect(before).toBe(16);
  });

  it("recorded age increments with elapsed years", () => {
    expect(ageFromRecorded(30, "2020-06-25 10:00:00", new Date(2026, 5, 25))).toBe(36);
    expect(ageFromRecorded(30, "2020-06-25 10:00:00", new Date(2026, 5, 24))).toBe(35);
  });
});

type EcTuple = [number, number, number];
type GcTuple = [number, number, number];
