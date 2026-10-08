import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
const root=new URL('../web/',import.meta.url);
const types={'/classification.html':'text/html','/classification.js':'text/javascript','/':'text/html','/index.html':'text/html','/app.js':'text/javascript','/style.css':'text/css','/hagane.wasm':'application/wasm','/nurbs.html':'text/html','/nurbs.js':'text/javascript','/surface.html':'text/html','/surface.js':'text/javascript','/viewer.js':'text/javascript'};
const server=createServer(async(req,res)=>{try{const path=new URL(req.url,'http://localhost').pathname;if(!types[path]){res.writeHead(404);res.end();return;}const data=await readFile(new URL(path==='/'?'index.html':path.slice(1),root));res.writeHead(200,{'Content-Type':types[path]});res.end(data);}catch{res.writeHead(500);res.end('Build the WASM module first.');}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
let browser;
try {
 const executable=process.env.HAGANE_CHROMIUM ?? (existsSync('/usr/bin/chromium')?'/usr/bin/chromium':undefined);
 browser=await chromium.launch({executablePath:executable,headless:true,args:['--enable-unsafe-swiftshader','--use-gl=angle','--use-angle=swiftshader']});
 const page=await browser.newPage({viewport:{width:1440,height:900},deviceScaleFactor:1});const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(`http://127.0.0.1:${server.address().port}/`);await page.waitForFunction(()=>window.haganeDemo?.ready);
 const volume=await page.evaluate(()=>window.haganeDemo.mesh.volume);assert.ok(Math.abs(volume-(115200-Math.PI*196*24))<1e-8);
 const initial=await page.locator('canvas').screenshot();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),initial);
 const canvas=await page.locator('canvas').boundingBox();await page.mouse.move(canvas.x+canvas.width/2,canvas.y+canvas.height/2);await page.mouse.down();await page.mouse.move(canvas.x+canvas.width/2+80,canvas.y+canvas.height/2+20,{steps:6});await page.mouse.up();const orbit=await page.locator('canvas').screenshot();assert.notDeepEqual(orbit,initial);
 await page.mouse.wheel(0,150);await page.waitForTimeout(100);assert.notDeepEqual(await page.locator('canvas').screenshot(),orbit);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(115200-Math.PI*24**2*24))<1e-8);assert.match(await page.locator('#radius-label').textContent(),/24.0/);
 const solid=await page.locator('canvas').screenshot();await page.locator('#wire').check();assert.notDeepEqual(await page.locator('canvas').screenshot(),solid);await page.locator('#wire').uncheck();
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});await page.locator('#reset').click();
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);
 assert.deepEqual(errors,[]);
 const presets=[{volume:115200-Math.PI*196*24,faces:7,edges:15},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:69336,faces:12,edges:30},{volume:Math.PI*704*24,faces:4,edges:6},{volume:115200-Math.PI*196*24,faces:10,edges:24},{volume:(4800-(4-Math.PI)*196)*24,faces:10,edges:24},{volume:(4600+(4-Math.PI)*9-Math.PI*196/2)*24,faces:16,edges:42},{volume:115200-Math.PI*49*24,faces:8,edges:18},{volume:(4800-(4-Math.PI)*576)*24,faces:13,edges:31},{volume:103680,faces:15,edges:45},{volume:5376*Math.PI,faces:11,edges:26},{volume:115200-3456*Math.PI,faces:13,edges:34},{volume:115200,faces:6,edges:13},{volume:67200,faces:6,edges:12},{volume:(4096-(32*Math.SQRT2-38)**2-(32*Math.SQRT2-42)**2-2*(32*Math.SQRT2-30)**2)*24,faces:10,edges:24}];
 for(let id=0;id<presets.length;id++){
  await page.locator('#preset').selectOption(String(id));const mesh=await page.evaluate(()=>window.haganeDemo.mesh);assert.ok(Math.abs(mesh.volume-presets[id].volume)<1e-8);assert.equal(mesh.faces,presets[id].faces);assert.equal(mesh.edges,presets[id].edges);assert.equal(await page.locator('#radius').isDisabled(),id===2);assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);
 }
 await page.locator('#preset').selectOption('5');
 assert.match(await page.locator('#radius-title').textContent(),/Corner radius/);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(4800-(4-Math.PI)*576)*24)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-mixed')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/mixed-profile.png',import.meta.url).pathname});}
 await page.locator('#preset').selectOption('6');
 assert.match(await page.locator('#radius-title').textContent(),/Notch radius/);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(4600+(4-Math.PI)*9-Math.PI*576/2)*24)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-region')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/mixed-region.png',import.meta.url).pathname});}
 await page.locator('#preset').selectOption('7');
 assert.match(await page.locator('#radius-title').textContent(),/Cut offset/);
 const splitBefore=await page.locator('canvas').screenshot();
 const splitPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(115200-Math.PI*49*24))<1e-8);
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),splitPositions);
 await page.locator('#wire').check();assert.notDeepEqual(await page.locator('canvas').screenshot(),splitBefore);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-split')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/face-split.png',import.meta.url).pathname});}
 await page.locator('#wire').uncheck();
 await page.locator('#preset').selectOption('8');
 assert.match(await page.locator('#radius-title').textContent(),/Arc cut offset/);
 const arcSplitPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),arcSplitPositions);
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(4800-(4-Math.PI)*576)*24)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-arc-split')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/arc-face-split.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('9');
 assert.match(await page.locator('#radius-title').textContent(),/Hole cut offset/);
 assert.match(await page.locator('#radius-label').textContent(),/-2.0/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 const holeCutPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),holeCutPositions);
 assert.equal(await page.evaluate(()=>window.haganeDemo.mesh.volume),103680);
 assert.match(await page.locator('#radius-label').textContent(),/8.0/);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-cut-graph')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/cut-graph.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('10');
 assert.match(await page.locator('#radius-title').textContent(),/Annular cut offset/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['8 mm','24 mm']);
 const repeatedArcPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),repeatedArcPositions);
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-5376*Math.PI)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-repeated-arcs')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/repeated-arc-split.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('11');
 assert.match(await page.locator('#radius-title').textContent(),/Periodic cut offset/);
 assert.match(await page.locator('#radius-label').textContent(),/-2.0/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 const periodicPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=16;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),periodicPositions);
 assert.ok(Math.abs(await page.evaluate(()=>window.haganeDemo.mesh.volume)-(115200-3456*Math.PI))<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-periodic')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/periodic-face-split.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('12');
 assert.match(await page.locator('#radius-title').textContent(),/Boundary subdivision/);
 assert.match(await page.locator('#radius-label').textContent(),/43.8%/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['25%','75%']);
 const sewnPositions=await page.evaluate(()=>window.haganeDemo.mesh.positions);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});
 assert.notDeepEqual(await page.evaluate(()=>window.haganeDemo.mesh.positions),sewnPositions);
 assert.equal(await page.evaluate(()=>window.haganeDemo.mesh.volume),115200);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-sewing')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/sewing.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('13');
 assert.match(await page.locator('#radius-title').textContent(),/Plane offset/);
 assert.match(await page.locator('#radius-label').textContent(),/-2.0/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 assert.equal(await page.evaluate(()=>window.haganeDemo.mesh.volume),67200);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});assert.equal(await page.evaluate(()=>window.haganeDemo.mesh.volume),19200);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-solid-split')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/solid-split.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('14');
 assert.match(await page.locator('#radius-title').textContent(),/Cutter offset/);
 assert.match(await page.locator('#radius-label').textContent(),/-2.0/);
 assert.deepEqual(await page.locator('.range-label span').allTextContents(),['-8 mm','8 mm']);
 const convexBefore=await page.evaluate(()=>window.haganeDemo.mesh);
 assert.equal(convexBefore.faces,10);
 await page.locator('#radius').evaluate(e=>{e.value=24;e.dispatchEvent(new Event('input'));});const convexAfter=await page.evaluate(()=>window.haganeDemo.mesh);assert.equal(convexAfter.faces,9);assert.notDeepEqual(convexAfter.positions,convexBefore.positions);
 const diamond=32*Math.SQRT2;assert.ok(Math.abs(convexAfter.volume-(4096-(diamond-32)**2-2*(diamond-30)**2)*24)<1e-8);
 await page.locator('#radius').evaluate(e=>{e.value=14;e.dispatchEvent(new Event('input'));});
 if(process.argv.includes('--capture-convex')){await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/convex-intersection.png',import.meta.url).pathname});await page.locator('#wire').uncheck();}
 await page.locator('#preset').selectOption('1');
 const unplaced=await page.evaluate(()=>window.haganeDemo.mesh);
 await page.locator('#preset').selectOption('4');
 const placed=await page.evaluate(()=>window.haganeDemo.mesh);
 assert.notDeepEqual(placed.positions,unplaced.positions);assert.notDeepEqual(placed.normals,unplaced.normals);
 if(process.argv.includes('--capture-placement')){await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/placement.png',import.meta.url).pathname});}
 await page.locator('#preset').selectOption('0');
 if(process.argv.includes('--capture')){
  const frames=await mkdtemp(join(tmpdir(),'hagane-demo-'));
  for(let i=0;i<64;i++){if(i%16===0)await page.locator('#preset').selectOption(String(Math.floor(i/16)));await page.evaluate(a=>window.haganeDemo.setAngle(a),0.65+0.8*(i%16)/16);await page.screenshot({path:join(frames,`frame-${String(i).padStart(3,'0')}.png`)});}
  const output=new URL('../docs/demo.gif',import.meta.url).pathname;
  const result=spawnSync('ffmpeg',['-y','-framerate','12','-i',join(frames,'frame-%03d.png'),'-filter_complex','[0:v]scale=1008:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3','-loop','0',output],{encoding:'utf8'});
  if(result.status!==0)throw Error(result.stderr);console.log('Recorded actual WASM/WebGL demo to docs/demo.gif (64 frames, four exact B-rep operations).');await rm(frames,{recursive:true,force:true});
 }
 await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 await page.setViewportSize({width:1440,height:900});
 await page.goto(`http://127.0.0.1:${server.address().port}/nurbs.html`);await page.waitForFunction(()=>window.haganeNurbs?.ready);
 let curve=await page.evaluate(()=>window.haganeNurbs.data);assert.ok(Math.abs(curve.point[0]-Math.SQRT1_2)<1e-14);assert.ok(Math.abs(curve.point[1]-Math.SQRT1_2)<1e-14);
 const circleImage=await page.locator('canvas').screenshot();
 await page.locator('#weight').evaluate(e=>{e.value=2;e.dispatchEvent(new Event('input'));});
 curve=await page.evaluate(()=>window.haganeNurbs.data);assert.ok(Math.abs(curve.point[0]-5/6)<1e-14);assert.notDeepEqual(await page.locator('canvas').screenshot(),circleImage);
 await page.locator('#parameter').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});curve=await page.evaluate(()=>window.haganeNurbs.data);assert.deepEqual(curve.point,[1,0,0]);assert.deepEqual(curve.tangent,[0,4,0]);
 await page.locator('#parameter').evaluate(e=>{e.value=0.5;e.dispatchEvent(new Event('input'));});await page.locator('#circle').click();
 curve=await page.evaluate(()=>window.haganeNurbs.data);assert.ok(Math.abs(curve.weight-Math.SQRT1_2)<1e-15);assert.ok(Math.abs(curve.point[0]-Math.SQRT1_2)<1e-14);
 if(process.argv.includes('--capture')){await page.screenshot({path:new URL('../docs/nurbs.png',import.meta.url).pathname});}
 await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.ok(await page.locator('canvas').isVisible());assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 await page.setViewportSize({width:1440,height:900});
 await page.goto(`http://127.0.0.1:${server.address().port}/surface.html`);await page.waitForFunction(()=>window.haganeSurface?.ready);
 let patch=await page.evaluate(()=>window.haganeSurface.data);assert.ok(Math.abs(patch.point[2]-8.75)<1e-13);assert.deepEqual(patch.normal,[0,0,1]);const initialPatch=await page.locator('canvas').screenshot();
 await page.locator('#height').evaluate(e=>{e.value=0;e.dispatchEvent(new Event('input'));});patch=await page.evaluate(()=>window.haganeSurface.data);assert.equal(patch.point[2],0);assert.notDeepEqual(await page.locator('canvas').screenshot(),initialPatch);
 await page.locator('#height').evaluate(e=>{e.value=35;e.dispatchEvent(new Event('input'));});await page.locator('#weight').evaluate(e=>{e.value=2;e.dispatchEvent(new Event('input'));});patch=await page.evaluate(()=>window.haganeSurface.data);assert.ok(Math.abs(patch.point[2]-14)<1e-13);
 await page.locator('#u').evaluate(e=>{e.value=0.3;e.dispatchEvent(new Event('input'));});await page.locator('#v').evaluate(e=>{e.value=0.7;e.dispatchEvent(new Event('input'));});patch=await page.evaluate(()=>window.haganeSurface.data);assert.ok(Math.abs(Math.hypot(...patch.normal)-1)<1e-13);assert.ok(Math.abs(patch.normal[0])+Math.abs(patch.normal[1])>0.01);
 const shaded=await page.locator('canvas').screenshot();await page.locator('#wire').check();assert.notDeepEqual(await page.locator('canvas').screenshot(),shaded);await page.locator('#wire').uncheck();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),shaded);
 if(process.argv.includes('--capture')){await page.locator('#height').evaluate(e=>{e.value=60;e.dispatchEvent(new Event('input'));});await page.locator('#weight').evaluate(e=>{e.value=2;e.dispatchEvent(new Event('input'));});await page.locator('#wire').check();await page.locator('#reset').click();await page.screenshot({path:new URL('../docs/nurbs-surface.png',import.meta.url).pathname});}
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 await page.setViewportSize({width:1440,height:900});
 await page.goto(`http://127.0.0.1:${server.address().port}/classification.html`);await page.waitForFunction(()=>window.haganeClassification?.ready);
 for(const [id,result] of [['material','inside'],['hole','outside'],['notch','outside'],['face','boundary'],['vertex','boundary'],['outside','outside']]){
  await page.locator('#probe').selectOption(id);assert.equal(await page.locator('#location').textContent(),result);assert.equal(await page.evaluate(()=>window.haganeClassification.data.location),result);
 }
 await page.locator('#probe').selectOption('material');
 await page.locator('#x').evaluate(e=>{e.value=-26;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#location').textContent(),'outside');assert.equal(await page.locator('#probe').inputValue(),'custom');
 await page.locator('#x').evaluate(e=>{e.value=-20;e.dispatchEvent(new Event('input'));});assert.equal(await page.locator('#location').textContent(),'boundary');
 await page.locator('#probe').selectOption('face');await page.locator('#wire').check();await page.locator('#reset').click();
 if(process.argv.includes('--capture-classification'))await page.screenshot({path:new URL('../docs/classification.png',import.meta.url).pathname});
 const classifierView=await page.locator('canvas').screenshot();await page.locator('canvas').focus();await page.keyboard.press('ArrowRight');assert.notDeepEqual(await page.locator('canvas').screenshot(),classifierView);
 assert.equal(await page.evaluate(()=>document.getElementById('view').getContext('webgl').getError()),0);await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),390);assert.deepEqual(errors,[]);
 console.log('Browser: solid point classification, material/hole/notch/boundary probes, sliders, orbit and responsive layout passed.');
 console.log('Browser: NURBS surface height/weight/UV, analytic point/normal, open-patch shading, wireframe, orbit and responsive layout passed.');
 console.log('Browser: NURBS weight/parameter controls, exact-circle reset, native-derived coordinates, canvas changes and responsive layout passed.');
 console.log('Browser: 15 B-rep presets, WASM generation, radius, keyboard/drag orbit, wheel zoom, wireframe, reset, responsive rendering passed.');
} finally {await browser?.close();server.close();}
