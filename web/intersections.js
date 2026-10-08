import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view'),{doubleSided:true});
 const response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;
 window.haganeIntersections={ready:false,data:null};
 function update(){try{
  const mode=Number($('mode').value),offset=Number($('offset').value);
  const scope=$('scope').value;
  const status=scope==='surface'?k.hagane_extrusion_intersections_demo(mode,offset,0):k.hagane_circular_face_intersections_demo(Number(scope),mode,offset,0);
  const data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  window.haganeIntersections.data=data;viewer.setMesh(data.display_mesh??data.mesh);
  $('part-title').textContent=scope==='surface'?'Skew circular extrusion':scope==='0'?'Upper semicircular face':'Lower semicircular face';
  const {intersection:hit,line}=data;
  const at=t=>line.anchor.map((v,i)=>v+line.direction[i]*t);
  const segments=(hit.endpoints??hit.hits).map(p=>[p.point,p.point.map((v,i)=>v+p.normal[i]*14)]);
  segments.push(mode===0?[at(0),at(40)]:[at(-0.2),at(1.2)]);viewer.setSegments(segments);
  $('kind').textContent=hit.kind==='points'?hit.hits[0].contact:hit.kind==='coincident'?'generator overlap':'empty';
  $('hits').textContent=String(hit.hits.length);$('parameters').textContent=(hit.range??hit.hits.map(p=>p.parameter)).map(t=>t.toFixed(3)).join(' / ')||'—';
  $('volume').textContent=data.mesh.volume.toLocaleString('en-US',{maximumFractionDigits:1})+' mm³';$('offset-label').value=offset.toFixed(1)+' mm';$('status').textContent=scope==='surface'?'✓ Line and surface agreement checked in Rust':'✓ Face trims and original edge parameters checked in Rust';
 }catch(e){window.haganeIntersections.data=null;viewer.setSegments([]);$('kind').textContent='unresolved';$('hits').textContent='—';$('parameters').textContent='—';$('status').textContent=e.message;}}
 $('probe').addEventListener('change',()=>{const p={crossing:[0,14],tangent:[0,24],empty:[0,30],generator:[1,0],reverse:[2,0]}[$('probe').value];if(p){$('mode').value=p[0];$('offset').value=p[1];update();}});
 for(const id of ['scope','mode','offset'])$(id).addEventListener(id==='offset'?'input':'change',()=>{$('probe').value='custom';update();});
 $('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('spin').addEventListener('change',()=>viewer.autoRotate=$('spin').checked);$('reset').addEventListener('click',()=>viewer.reset());update();window.haganeIntersections.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
