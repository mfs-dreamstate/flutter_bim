#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
#
Pod::Spec.new do |s|
  s.name             = 'flutter_bim'
  s.version          = '0.1.0'
  s.summary          = 'A high-performance BIM viewer for Flutter with IFC support.'
  s.description      = <<-DESC
A high-performance BIM (Building Information Modeling) viewer for Flutter with IFC file support,
3D rendering powered by Rust and wgpu, and element inspection capabilities.
                       DESC
  s.homepage         = 'https://github.com/mfs-dreamstate/flutter_bim'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Your Company' => 'email@example.com' }
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.public_header_files = 'Classes/**/*.h'
  s.dependency 'Flutter'
  s.platform = :ios, '12.0'
  s.static_framework = true

  # Rust static library
  s.vendored_libraries = 'Runner/librust.a'
  s.xcconfig = {
    'LIBRARY_SEARCH_PATHS' => '$(inherited) $(PODS_TARGET_SRCROOT)/Runner',
    'OTHER_LDFLAGS' => '$(inherited) -lrust'
  }

  # Flutter.framework does not contain a i386 slice.
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES', 'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386' }
  s.swift_version = '5.0'
end
