import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const script=readFileSync(new URL('../web/official.js',import.meta.url),'utf8');
function screen(fail=false) {
 const ids=new Map(['open-music','open-browser','official-status'].map(id=>[id,{disabled:false,textContent:''}]));
 const calls=[];
 vm.runInNewContext(script,{document:{getElementById:id=>ids.get(id)},window:{__TAURI__:{core:{invoke:async(...args)=>{calls.push(args);if(fail)throw new Error('private internal detail');}}}}});
 return {ids,calls};
}
test('official site launch and browser fallback need no credentials or user-supplied URL',async()=>{
 const {ids,calls}=screen();
 await ids.get('open-music').onclick(); await ids.get('open-browser').onclick();
 assert.deepEqual(calls,[['open_music'],['open_music_browser']]);
 assert.equal(ids.get('open-music').disabled,false);
});
test('a failed webview launch offers the browser fallback without leaking internals',async()=>{
 const {ids}=screen(true); await ids.get('open-music').onclick();
 assert.match(ids.get('official-status').textContent,/Open in browser/);
 assert.doesNotMatch(ids.get('official-status').textContent,/private/);
});
