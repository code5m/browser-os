"""Synthetic rejection cases for the manual observer; no GUI launch or I/O."""
import importlib.util
from pathlib import Path
import unittest
from PIL import Image, ImageDraw

spec = importlib.util.spec_from_file_location("visibility_observer",
    Path(__file__).resolve().parents[1] / "scripts/native-visibility-installed.py")
observer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(observer)

PALETTES = {
    "a": [(240,24,24),(24,240,24),(24,24,240),(240,240,24)],
    "b": [(24,240,240),(240,24,240),(240,96,24),(96,240,24)],
    "c": [(24,96,240),(24,240,96),(96,24,240),(240,24,96)],
    "d": [(240,168,24),(168,240,24),(24,168,240),(24,240,168)],
}
BOXES = [(80,180,400,420),(450,180,770,420)]


def paint(frame, box, name, swap=False):
    x0,y0,x1,y1 = box
    mx,my = (x0+x1)//2,(y0+y1)//2
    colors = PALETTES[name].copy()
    if swap:
        colors[0],colors[1] = colors[1],colors[0]
    draw = ImageDraw.Draw(frame)
    for rect,color in zip([(x0,y0,mx-1,my-1),(mx,y0,x1-1,my-1),
                           (x0,my,mx-1,y1-1),(mx,my,x1-1,y1-1)],colors):
        draw.rectangle(rect, fill=color)


class PixelRejection(unittest.TestCase):
    def test_wrong_quadrants_blank_frame_and_stale_marker_fail(self):
        base=Image.new("RGB",(1024,768),(36,36,36))
        ImageDraw.Draw(base).rectangle((10,10,70,120), fill=(100,100,100))
        def score(image, markers, baseline):
            ok, _, ghosts, _ = observer.score_pixels(image,BOXES,markers,PALETTES,Image)
            return observer.frame_matches(ok,ghosts,baseline), ghosts
        passed, baseline=score(base,[None,None],None)
        self.assertTrue(passed)
        self.assertFalse(score(base,["a",None],baseline)[0])
        shown=base.copy()
        paint(shown,BOXES[0],"a")
        self.assertTrue(score(shown,["a",None],baseline)[0])
        self.assertFalse(score(shown,[None,None],baseline)[0])
        self.assertFalse(score(shown,[None,"a"],baseline)[0])
        wrong=base.copy()
        paint(wrong,BOXES[0],"a",swap=True)
        self.assertFalse(score(wrong,["a",None],baseline)[0])
        ghost=shown.copy()
        paint(ghost,(680,470,1000,710),"a")
        self.assertFalse(score(ghost,["a",None],baseline)[0])
        other=shown.copy()
        paint(other,BOXES[1],"b")
        self.assertFalse(score(other,["a",None],baseline)[0])
        self.assertTrue(score(other,["a","b"],baseline)[0])

    def test_uniform_desktop_and_contaminated_baseline_are_not_absence_passes(self):
        for color in ((0,0,0),(36,36,36)):
            frame=Image.new("RGB",(1024,768),color)
            with self.assertRaisesRegex(RuntimeError, "uniform"):
                observer.score_pixels(frame,BOXES,[None,None],PALETTES,Image)
        ghosts={name:[0,0,0,0] for name in PALETTES}
        ghosts["a"][0]=46
        self.assertFalse(observer.frame_matches(True,ghosts,None))

    def test_terminal_observer_snapshot_blocks_late_stage(self):
        fixture = observer.Observer("nonce", PALETTES, None, None, None, Image, None, None)
        frozen = fixture.freeze()
        self.assertIsNotNone(frozen)
        self.assertEqual(frozen["stages"], [])
        with self.assertRaisesRegex(RuntimeError, "already terminal"):
            fixture.handle_stage({})
        with self.assertRaisesRegex(RuntimeError, "duplicate terminal"):
            fixture.handle_finished({})
        self.assertEqual(frozen["stages"], [])

    def test_out_of_screen_is_not_an_absence_pass(self):
        frame=Image.new("RGB",(1024,768),(0,0,0))
        with self.assertRaises(RuntimeError):
            observer.score_pixels(frame,[(-10,180,310,420),BOXES[1]],
                                  [None,None],PALETTES,Image)


if __name__ == "__main__":
    unittest.main()
