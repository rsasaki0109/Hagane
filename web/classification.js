import {createMeshViewer} from './viewer.js';
const $=id=>document.getElementById(id);
try{
 const viewer=createMeshViewer($('view'));
 const response=await fetch('hagane.wasm');if(!response.ok)throw Error('Build WASM first: ./scripts/build-web.sh');const {instance}=await WebAssembly.instantiate(await response.arrayBuffer(),{}),k=instance.exports;
 const read=status=>{const data=JSON.parse(new TextDecoder().decode(new Uint8Array(k.memory.buffer,k.hagane_output_ptr(),k.hagane_output_len())));if(status)throw Error(data.error);return data;};
 viewer.setMesh(read(k.hagane_classification_mesh()));
 window.haganeClassification={ready:false,data:null};
 function query(){const p=['x','y','z'].map(id=>Number($(id).value));const data=read(k.hagane_classify_demo(...p));window.haganeClassification.data=data;$('location').textContent=data.location;['x','y','z'].forEach((id,i)=>$(id+'-label').value=p[i].toFixed(1)+' mm');viewer.setMarker(p,[0,0,1]);$('status').textContent='✓ Classified against exact faces in Rust';}
 function update(){try{query();}catch(e){$('location').textContent='unresolved';$('status').textContent=e.message;}}
 for(const id of ['x','y','z'])$(id).addEventListener('input',()=>{$('probe').value='custom';update();});
 const probes={material:[-10,0,0],hole:[-26,0,0],notch:[10,10,0],face:[-10,0,12],vertex:[-40,-30,-12],outside:[45,0,0]};
 $('probe').addEventListener('change',()=>{probes[$('probe').value]?.forEach((v,i)=>$(['x','y','z'][i]).value=v);update();});
 $('wire').addEventListener('change',()=>viewer.wireframe=$('wire').checked);$('spin').addEventListener('change',()=>viewer.autoRotate=$('spin').checked);$('reset').addEventListener('click',()=>viewer.reset());query();window.haganeClassification.ready=true;
}catch(e){$('status').textContent=e.message;console.error(e);}
