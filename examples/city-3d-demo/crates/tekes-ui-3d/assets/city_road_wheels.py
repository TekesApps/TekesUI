"""Original circular road wheelsets, separate from car bodies and reused on GPU."""
import math
from block_geometry import Geometry

def add_road_wheels(a,plain):
    for name,y,r,x,width,z in [
        ('sedan-rear',.21*.78,.17*.78,.39*.90,.075*.90,-.49),
        ('sedan-front',.21*.78,.17*.78,.39*.90,.075*.90,.51),
        ('heavy-front',.22,.19,.44,.09,.69),
        ('ambulance-rear',.22,.19,.44,.09,-.72),
        ('heavy-middle',.22,.19,.44,.09,-.55),
        ('heavy-rear',.22,.19,.44,.09,-1.08),
    ]:
        g=Geometry(a)
        for side in [-1,1]:
            face=side*(x+width/2+.007)
            g.cylinder(side*x,y,z,r,width,plain,(30,36,40),axis='x',segments=32,smooth=True)
            g.cylinder(face,y,z,r*.61,.014,plain,(139,151,155),axis='x',segments=24,smooth=True)
            for j in range(5):
                angle=j*math.tau/5
                g.cylinder(face+side*.010,y+r*.39*math.cos(angle),z+r*.39*math.sin(angle),r*.045,.009,plain,(65,77,84),axis='x',segments=8,smooth=True)
            # A subtle tire inspection mark makes rotation visible at close range.
            g.beam((face+side*.012,y+r*.78,z),(face+side*.012,y+r*.93,z),r*.055,plain,(150,154,148))
        g.finish('road-wheel-'+name)
