"""Original low-poly volumetric clouds, shared as immutable indexed geometry."""
from block_geometry import Geometry

def add_clouds(a):
    mat=a.material('Soft cloud volume',(245,247,250))
    g=Geometry(a)
    for x,y,z,rx,ry,rz in [(-.9,0,0,.9,.24,.52),(0,.16,0,1.05,.43,.63),(.85,.02,.04,.77,.30,.49),(-.25,.09,-.40,.77,.27,.52),(.30,-.06,.40,.83,.22,.48)]:
        g.ellipsoid(x,y,z,rx,ry,rz,mat,sides=16,rings=8)
    g.finish('sky-cloud')
