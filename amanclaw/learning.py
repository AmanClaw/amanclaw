"""
Learning Engine — re-exported from standalone amanclaw-learning package.

All imports from amanclaw.learning continue to work.
"""

from amanclaw_learning import LearningEngine, MemoryBackend
from amanclaw_learning.patterns import CORRECTION_PATTERNS, TEACHING_PATTERNS

__all__ = ["LearningEngine", "MemoryBackend", "CORRECTION_PATTERNS", "TEACHING_PATTERNS"]
