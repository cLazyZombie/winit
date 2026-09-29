# iOS scene 지원 (0.30.13 fork)

2026-09-29 조사: upstream stable v0.30.13, prerelease v0.31.0-beta.3와 master
`8b5f46d4be7d29f5b0ed91f4343b78d8c2668bd0`에는 UIScene 지원이 없다.
[issue #4224](https://github.com/rust-windowing/winit/issues/4224)는 열려 있다.
objc2 0.6 이행은 [PR #4092](https://github.com/rust-windowing/winit/pull/4092),
commit `953d9b426886749e2f88250f420c87db58080c97`의 API 이행을 참고했다.
해당 변경은 scene 구현이 아니며, 이 fork는 iOS backend에만 필요한 이행을 적용한다.
macOS backend와 objc2 0.5 의존성, 기존 한국어 IME·Metal gravity 변경은 보존한다.
iOS 의존성의 MSRV는 Rust 1.71이다(실제 검증은 현재 프로젝트 toolchain).

앱의 Info.plist에 다음을 넣는다. 기존 app/delegate를 교체하지 않으며,
manifest가 없는 앱은 기존 UIApplication 알림 경로를 사용한다.

```xml
<key>UIApplicationSceneManifest</key>
<dict>
  <key>UIApplicationSupportsMultipleScenes</key><false/>
  <key>UISceneConfigurations</key>
  <dict><key>UIWindowSceneSessionRoleApplication</key><array><dict>
    <key>UISceneConfigurationName</key><string>Default</string>
    <key>UISceneClassName</key><string>UIWindowScene</string>
  </dict></array></dict>
</dict>
```

scene 활성화는 Resumed, 마지막 활성 scene의 비활성화/연결 해제는 Suspended로 변환한다.
중복 알림은 제거한다. Window는 Resumed 이후 생성해야 하며 활성 UIWindowScene에 연결된다.
여러 활성 scene이 있으면 session ID 순 첫 scene을 사용한다. scene별 창 배정 API와
external display/multi-window UX는 이번 단일 scene 이식 범위에 포함하지 않는다.

검증: `cargo test --test ios_scene_lifecycle`, macOS `cargo test --lib --tests`,
`cargo clippy --all-targets -- -D warnings`, iOS target check와 실제 앱 screenshot.
