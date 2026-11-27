#!/usr/bin/env python3
"""tools/engine/script-risks.py's line scan on the two scripts that broke duels (2026-11-25) and their fixed overrides,
written out here so the test needs no pinned data. Standard library only."""
import importlib.util
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("script_risks", HERE / "script-risks.py")
risks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(risks)

EQUIP = """function s.cost(e,tp,eg,ep,ev,re,r,rp,chk) end
e1:SetCost(s.cost)
function s.op(e,tp,eg,ep,ev,re,r,rp)
	local tc=Duel.GetFirstTarget()
	if tc:IsRelateToEffect(e) and tc:IsFaceup() then
		Duel.Equip(tp,e:GetHandler(),tc)
	end
end
"""
CATAPULT = """e2:SetCost(s.cost)
function s.sttg(e,tp,eg,ep,ev,re,r,rp,chk)
	local g=Duel.SelectMatchingCard(tp,aux.FaceupFilter(Card.IsSpellTrap),tp,0,LOCATION_ONFIELD,1,1,nil)
	Duel.SetTargetCard(g:GetFirst())
end
"""


class ScriptRisks(unittest.TestCase):
    def test_a_first_card_used_unchecked_after_a_cost(self):
        self.assertEqual(risks.risks(EQUIP), [("first card unchecked", [4])])
        self.assertEqual(risks.risks(CATAPULT), [("first card unchecked", [4])])

    def test_the_fixed_scripts_pass(self):
        fixed = EQUIP.replace("if tc:IsRelateToEffect(e)", "if tc and tc:IsRelateToEffect(e)")
        self.assertEqual(risks.risks(fixed), [])
        self.assertEqual(risks.risks(CATAPULT.replace("\tDuel.SetTargetCard", "\tif #g>0 then Duel.SetTargetCard").replace("GetFirst())\n", "GetFirst()) end\n")), [])

    def test_no_cost_no_risk(self):
        # A target chosen as the card is activated stays: without a cost nothing can leave the choice empty.
        self.assertEqual(risks.risks(EQUIP.replace("e1:SetCost(s.cost)\n", "")), [])

    def test_an_effect_that_widens_materials(self):
        text = "\tlocal e1=Effect.CreateEffect(c)\n\te1:SetCode(EFFECT_HAND_SYNCHRO)\n"
        self.assertEqual(risks.risks(text), [("widens materials", [2])])
        self.assertEqual(risks.risks("--EFFECT_HAND_SYNCHRO in a comment\n"), [])

    def test_a_timing_the_core_never_raises(self):
        # Attack Cannon and Dizzy Tiger waited for EVENT_BATTLE_END: deprecated, so their effects never happened.
        self.assertEqual(risks.risks("\te1:SetCode(EVENT_BATTLE_END)\n"), [("never raised", [1])])
        self.assertEqual(risks.risks("\te1:SetCode(EVENT_DAMAGE_STEP_END)\n"), [])


if __name__ == "__main__":
    unittest.main()
