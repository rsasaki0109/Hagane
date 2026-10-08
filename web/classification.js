import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view'));
 const response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;
 const read=status=>{const data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);return data;};
 window.haganeClassification={ready:false,data:null,model:0};
 const models=[
  {name:'Concave plate',description:'Exact planar solid · polygon through-hole',probes:{material:[-10,0,0],hole:[-26,0,0],notch:[10,10,0],face:[-10,0,12],vertex:[-40,-30,-12],outside:[45,0,0]}},
  {name:'Plate with circular bore',description:'Exact planes and cylinder wall · Ø28 mm through-hole',probes:{material:[-20,0,0],hole:[0,0,0],notch:[14,0,0],face:[-20,0,12],vertex:[-40,-30,-12],outside:[45,0,0]}},
  {name:'Cylinder',description:'Exact circular caps and periodic wall · Ø48 mm',probes:{material:[12,0,0],hole:[0,0,0],notch:[24,0,0],face:[12,0,12],vertex:[24,0,12],outside:[35,0,0]}},
  {name:'Hollow tube',description:'Exact annular caps · Ø48 / Ø28 mm',probes:{material:[20,0,0],hole:[0,0,0],notch:[14,0,0],face:[20,0,12],vertex:[24,0,12],outside:[35,0,0]}},
  {name:'Rounded plate',description:'Exact bounded arcs and partial cylinder walls',probes:{material:[0,0,0],hole:[39,29,0],notch:[40,16,0],face:[0,0,12],vertex:[40,16,12],outside:[45,0,0]}},
  {name:'Arc notch and rounded hole',description:'Concave circular notch · rounded through-hole',probes:{material:[-20,0,0],hole:[0,-10,0],notch:[0,16,0],face:[-20,0,12],vertex:[-40,-30,-12],outside:[45,0,0]}},
  {name:'Skew plate with rounded hole',description:'Exact circular translation walls · rounded through-hole',probes:{material:[26,-3,0],hole:[6,-3,0],notch:[34,19,0],face:[32,-6,12],vertex:[44,8,12],outside:[45,30,0]}},
  {name:'Oblique sections with rounded hole',description:'Exact ellipse edges · harmonic circular wall trims',probes:{material:[26,-3,0],hole:[6,-3,0],notch:[34,19,0],face:[32,-6,12],vertex:[44,8,12],outside:[45,30,0]}}
 ];
 function query(){const model=Number($('model').value),p=['x','y','z'].map(id=>Number($(id).value));const data=read(model===0?k.hagane_classify_demo(...p):k.hagane_classify_curved_demo(model-1,...p));window.haganeClassification.data=data;window.haganeClassification.model=model;$('location').textContent=data.location;$('volume').textContent=data.volume.toLocaleString('en-US',{maximumFractionDigits:1})+' mm³';['x','y','z'].forEach((id,i)=>$(id+'-label').value=p[i].toFixed(1)+' mm');viewer.setMarker(p,[0,0,1]);$('status').textContent='✓ Classified against exact faces in Rust';}
 function update(){try{query();}catch(e){window.haganeClassification.data=null;$('location').textContent='unresolved';$('status').textContent=e.message;}}
 function probe(){models[Number($('model').value)].probes[$('probe').value]?.forEach((v,i)=>$(['x','y','z'][i]).value=v);update();}
 function model(){try{const id=Number($('model').value),data=models[id];viewer.setMesh(read(id===0?k.hagane_classification_mesh():k.hagane_curved_classification_mesh(id-1)));$('part-title').textContent=data.name;$('part-description').textContent=data.description;document.querySelector('#probe option[value="hole"]').textContent=id===2?'On solid axis':id===4?'Outside rounded corner':'Inside through-hole';document.querySelector('#probe option[value="notch"]').textContent=id===0?'Inside concave notch':id>=6?'On curved wall':'On cylinder wall';document.querySelector('#probe option[value="vertex"]').textContent=id<2||id===5?'On corner vertex':id===4||id>=6?'On arc endpoint':'On circular rim';$('probe').value='material';probe();}catch(e){$('status').textContent=e.message;}}
 for(const id of ['x','y','z'])$(id).addEventListener('input',()=>{$('probe').value='custom';update();});
 $('probe').addEventListener('change',probe);$('model').addEventListener('change',model);
 $('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('spin').addEventListener('change',()=>viewer.autoRotate=$('spin').checked);$('reset').addEventListener('click',()=>viewer.reset());model();window.haganeClassification.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
