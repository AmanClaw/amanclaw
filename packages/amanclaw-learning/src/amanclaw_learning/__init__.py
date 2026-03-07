"""
amanclaw-learning — Self-learning engine for AI assistants.

Usage:
    from amanclaw_learning import LearningEngine, MemoryBackend

    class MyStorage:
        # Implement MemoryBackend protocol methods
        ...

    engine = LearningEngine(MyStorage())
    engine.is_correction("No, I meant JavaScript")
"""

from amanclaw_learning.backend import MemoryBackend
from amanclaw_learning.engine import LearningEngine

__all__ = ["LearningEngine", "MemoryBackend"]
