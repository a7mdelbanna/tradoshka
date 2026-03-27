import sys
import os

# Allow `from ai_predictor.xxx import ...` by adding the tradoshka_poly dir to sys.path
# __file__ = .../tradoshka_poly/ai_predictor/tests/conftest.py
# parent of ai_predictor is tradoshka_poly, so go up two levels from __file__
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..")))
