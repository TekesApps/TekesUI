# Local SimCity 4 asset experiment

User requested a local experiment using their installed game as a complexity/style reference. Source application was read-only: `/Applications/Sim City 4 Deluxe Edition.app`. All extracted textures, converted models, experiment scripts and apps are under `/tmp/tekesui-sc4-lab/`, outside the source tree. They are not included in the Gallery or original City Block assets.

The installed DBPF archives contain S3D meshes and FSH textures. Selected native meshes are view-dependent exteriors, not four ordinary self-contained pictures. The experiment reads DBPF index/QFS data, native vertex/index/material groups and DXT textures, rotates four directional meshes into a common frame and selects exterior faces by direction. It preserves indexed geometry and exports GLB for the existing wgpu viewer, without adding a new rendering framework or runtime parser.

Format inspection reference: [OpenSC4 S3D reader](https://github.com/cfpi-fpsi/OpenSC4/blob/main/addons/dbpf/S3DSubfile.gd) and adjacent DBPF/FSH readers. These served as format references; no game/parser dependency was added to TekesUI.

Two selected examples passed local native display and alternate-angle snapshot inspection:

- Small New York office: model IDs 05f60400 / 05f60410 / 05f60420 / 05f60430; 112 assembled exterior triangles, approximately 240 KB GLB.
- Chicago office tower: 059f0400 family, six material/vertex/index groups per directional mesh; approximately 1.6 MB GLB.

The independent `TekesUI SC4 3D Lab.app` defaults to the Chicago tower, while `TekesUI SC4 Office Lab.app` shows the small office. Both use the existing compiled viewer. Local launchers and app signatures were generated for this experiment. Source package/type inventories, texture provenance and conversion receipts accompany the models.

This is reuse/assembly of native approximate geometry, not AI recovery of original high-poly authoring models. Source lighting and small details remain baked into low-resolution textures, and view-dependent seams persist. A separate Houston tower trial exposed unsuitable roof/billboard artifacts and was not selected for the demonstration. Original district complexity was subsequently increased using original procedural geometry/textures in independent offline modules.
