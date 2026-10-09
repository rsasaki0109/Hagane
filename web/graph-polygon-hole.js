import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view')),response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');
 const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;window.haganeGraphPolygonHole={ready:false,data:null};
 function generate(){
  const parse=id=>{const p=JSON.parse($(id).value);if(!Array.isArray(p)||p.length<3||p.length>16||p.some(pair=>!Array.isArray(pair)||pair.length!==2||pair.some(x=>typeof x!=='number'||!Number.isFinite(x))))throw Error('Boundaries require 3–16 finite numeric UV pairs.');return p;};
  const polygon=$('custom-polygon').checked?parse('polygon'):[],opening=parse('opening');
  const values=['width','depth','height','bulge','surface-error'].map(id=>Number($(id).value));values.push(Number($('angle').value)*Math.PI/180,...['tx','ty','tz','u-min','u-max','v-min','v-max'].map(id=>Number($(id).value)),polygon.length,opening.length,...polygon.flat(),...opening.flat());
  k.hagane_graph_polygon_hole_begin();for(const value of values)if(k.hagane_graph_polygon_hole_push(value))throw Error('Polygon graph transport rejected nonfinite or excessive input.');
  const status=k.hagane_graph_polygon_hole_finish(),data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);
  const center=[0,1,2].map(axis=>{let lo=Infinity,hi=-Infinity;for(let i=axis;i<data.positions.length;i+=3){lo=Math.min(lo,data.positions[i]);hi=Math.max(hi,data.positions[i]);}return (lo+hi)/2;});
  const segments=[];for(const samples of data.boundary_samples)for(let i=3;i<samples.length;i+=3)segments.push([samples.slice(i-3,i),samples.slice(i,i+3)]);
  viewer.setMesh({positions:data.positions.map((x,i)=>x-center[i%3]),normals:data.normals});viewer.setSegments(segments.map(segment=>segment.map(point=>point.map((x,i)=>x-center[i]))));window.haganeGraphPolygonHole.data=data;
  $('genus').textContent='1';$('cap-wires').textContent=data.brep.wire_edges.slice(0,2).map(w=>w.length).join(' / ');$('surface-error-label').value=data.error.toFixed(3);$('volume').textContent=data.volume.toFixed(3)+' mm³';$('centroid').textContent=data.mass_properties.centroid.map(x=>x.toFixed(4)).join(', ');$('faces').textContent=String(data.brep.faces);$('edges').textContent=String(data.brep.edges);$('triangles').textContent=String(data.mesh.triangles.length);$('surface-bound').textContent=data.error_bounds.reduce((maximum,bound)=>Math.max(maximum,bound),0).toExponential(3);$('status').textContent='✓ Closed genus-one B-rep validated · actual inner walls · annular caps';
 }
 const update=()=>{try{generate();}catch(e){$('status').textContent='Polygon opening rejected: '+e.message;}};
 $('apply').addEventListener('click',update);$('surface-error').addEventListener('input',update);$('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('reset').addEventListener('click',()=>viewer.reset());generate();window.haganeGraphPolygonHole.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
