from dataclasses import dataclass


@dataclass
class Reading:
    hour: int
    height_m: float


def high_water(readings):
    return max(readings, key=lambda r: r.height_m)


def low_water(readings):
    return min(readings, key=lambda r: r.height_m)


def tidal_range(readings):
    return high_water(readings).height_m - low_water(readings).height_m


def safe_hours(readings, draft_m):
    return [r.hour for r in readings if r.height_m >= draft_m]
