"""Paint configuration, separate from E235 body dimensions and geometry.
Future liveries can replace these slots without changing the shell/interior code.
"""
from dataclasses import dataclass
@dataclass(frozen=True)
class TrainLivery:
    name:str
    stainless:tuple
    line_green:tuple
    frame:tuple
YAMANOTE_E235=TrainLivery('Yamanote E235',(202,212,216),(138,204,53),(139,160,169))
