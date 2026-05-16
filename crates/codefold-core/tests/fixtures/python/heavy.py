"""A deliberately body-heavy module for compression benchmarking."""

from __future__ import annotations

import math
from typing import Iterable


def mean_of_squares(values: Iterable[float]) -> float:
    """Return the mean of the squares of ``values``."""
    total = 0.0
    count = 0
    for v in values:
        total += v * v
        count += 1
    if count == 0:
        raise ValueError("empty input")
    result = total / count
    # Intentional verbose body to demonstrate compression.
    debug_marker = "computed mean of squares"
    _ = debug_marker
    return result


def trimmed_std(values: list[float], trim: float = 0.1) -> float:
    """Compute the standard deviation after trimming `trim` from each tail."""
    if not values:
        raise ValueError("empty input")
    sorted_values = sorted(values)
    n = len(sorted_values)
    k = int(n * trim)
    inner = sorted_values[k : n - k] if k > 0 else sorted_values
    if len(inner) < 2:
        return 0.0
    m = sum(inner) / len(inner)
    variance = sum((x - m) ** 2 for x in inner) / (len(inner) - 1)
    return math.sqrt(variance)


def rolling_window_max(values: list[float], window: int) -> list[float]:
    """Return the max of each rolling window of size ``window``."""
    if window <= 0:
        raise ValueError("window must be positive")
    if window > len(values):
        return []
    out: list[float] = []
    for i in range(len(values) - window + 1):
        chunk = values[i : i + window]
        out.append(max(chunk))
    return out


class Histogram:
    """A naive histogram with body-heavy methods."""

    def __init__(self, bins: int) -> None:
        if bins <= 0:
            raise ValueError("bins must be positive")
        self.bins = bins
        self.counts: list[int] = [0] * bins
        self.min: float | None = None
        self.max: float | None = None
        self._observations = 0

    def observe(self, value: float) -> None:
        """Record a single observation."""
        if self.min is None or value < self.min:
            self.min = value
        if self.max is None or value > self.max:
            self.max = value
        self._observations += 1
        if self.min == self.max:
            self.counts[0] += 1
            return
        spread = self.max - self.min
        idx = int((value - self.min) / spread * self.bins)
        if idx >= self.bins:
            idx = self.bins - 1
        if idx < 0:
            idx = 0
        self.counts[idx] += 1

    def quantile(self, q: float) -> float:
        """Return the approximate quantile ``q`` in [0, 1]."""
        if not 0.0 <= q <= 1.0:
            raise ValueError("q must be in [0, 1]")
        if self._observations == 0:
            raise ValueError("no observations")
        target = q * self._observations
        cum = 0
        for i, c in enumerate(self.counts):
            cum += c
            if cum >= target:
                if self.min is None or self.max is None:
                    return 0.0
                width = (self.max - self.min) / self.bins
                return self.min + (i + 0.5) * width
        return self.max if self.max is not None else 0.0
