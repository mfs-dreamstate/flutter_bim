// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'entities.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$IfcValue {
  @override
  bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType && other is IfcValue);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'IfcValue()';
  }
}

/// @nodoc
class $IfcValueCopyWith<$Res> {
  $IfcValueCopyWith(IfcValue _, $Res Function(IfcValue) __);
}

/// Adds pattern-matching-related methods to [IfcValue].
extension IfcValuePatterns on IfcValue {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(IfcValue_Null value)? null_,
    TResult Function(IfcValue_Integer value)? integer,
    TResult Function(IfcValue_Real value)? real,
    TResult Function(IfcValue_String value)? string,
    TResult Function(IfcValue_Enum value)? enum_,
    TResult Function(IfcValue_Boolean value)? boolean,
    TResult Function(IfcValue_EntityRef value)? entityRef,
    TResult Function(IfcValue_List value)? list,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case IfcValue_Null() when null_ != null:
        return null_(_that);
      case IfcValue_Integer() when integer != null:
        return integer(_that);
      case IfcValue_Real() when real != null:
        return real(_that);
      case IfcValue_String() when string != null:
        return string(_that);
      case IfcValue_Enum() when enum_ != null:
        return enum_(_that);
      case IfcValue_Boolean() when boolean != null:
        return boolean(_that);
      case IfcValue_EntityRef() when entityRef != null:
        return entityRef(_that);
      case IfcValue_List() when list != null:
        return list(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(IfcValue_Null value) null_,
    required TResult Function(IfcValue_Integer value) integer,
    required TResult Function(IfcValue_Real value) real,
    required TResult Function(IfcValue_String value) string,
    required TResult Function(IfcValue_Enum value) enum_,
    required TResult Function(IfcValue_Boolean value) boolean,
    required TResult Function(IfcValue_EntityRef value) entityRef,
    required TResult Function(IfcValue_List value) list,
  }) {
    final _that = this;
    switch (_that) {
      case IfcValue_Null():
        return null_(_that);
      case IfcValue_Integer():
        return integer(_that);
      case IfcValue_Real():
        return real(_that);
      case IfcValue_String():
        return string(_that);
      case IfcValue_Enum():
        return enum_(_that);
      case IfcValue_Boolean():
        return boolean(_that);
      case IfcValue_EntityRef():
        return entityRef(_that);
      case IfcValue_List():
        return list(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(IfcValue_Null value)? null_,
    TResult? Function(IfcValue_Integer value)? integer,
    TResult? Function(IfcValue_Real value)? real,
    TResult? Function(IfcValue_String value)? string,
    TResult? Function(IfcValue_Enum value)? enum_,
    TResult? Function(IfcValue_Boolean value)? boolean,
    TResult? Function(IfcValue_EntityRef value)? entityRef,
    TResult? Function(IfcValue_List value)? list,
  }) {
    final _that = this;
    switch (_that) {
      case IfcValue_Null() when null_ != null:
        return null_(_that);
      case IfcValue_Integer() when integer != null:
        return integer(_that);
      case IfcValue_Real() when real != null:
        return real(_that);
      case IfcValue_String() when string != null:
        return string(_that);
      case IfcValue_Enum() when enum_ != null:
        return enum_(_that);
      case IfcValue_Boolean() when boolean != null:
        return boolean(_that);
      case IfcValue_EntityRef() when entityRef != null:
        return entityRef(_that);
      case IfcValue_List() when list != null:
        return list(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? null_,
    TResult Function(PlatformInt64 field0)? integer,
    TResult Function(double field0)? real,
    TResult Function(String field0)? string,
    TResult Function(String field0)? enum_,
    TResult Function(bool field0)? boolean,
    TResult Function(int field0)? entityRef,
    TResult Function(List<IfcValue> field0)? list,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case IfcValue_Null() when null_ != null:
        return null_();
      case IfcValue_Integer() when integer != null:
        return integer(_that.field0);
      case IfcValue_Real() when real != null:
        return real(_that.field0);
      case IfcValue_String() when string != null:
        return string(_that.field0);
      case IfcValue_Enum() when enum_ != null:
        return enum_(_that.field0);
      case IfcValue_Boolean() when boolean != null:
        return boolean(_that.field0);
      case IfcValue_EntityRef() when entityRef != null:
        return entityRef(_that.field0);
      case IfcValue_List() when list != null:
        return list(_that.field0);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() null_,
    required TResult Function(PlatformInt64 field0) integer,
    required TResult Function(double field0) real,
    required TResult Function(String field0) string,
    required TResult Function(String field0) enum_,
    required TResult Function(bool field0) boolean,
    required TResult Function(int field0) entityRef,
    required TResult Function(List<IfcValue> field0) list,
  }) {
    final _that = this;
    switch (_that) {
      case IfcValue_Null():
        return null_();
      case IfcValue_Integer():
        return integer(_that.field0);
      case IfcValue_Real():
        return real(_that.field0);
      case IfcValue_String():
        return string(_that.field0);
      case IfcValue_Enum():
        return enum_(_that.field0);
      case IfcValue_Boolean():
        return boolean(_that.field0);
      case IfcValue_EntityRef():
        return entityRef(_that.field0);
      case IfcValue_List():
        return list(_that.field0);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? null_,
    TResult? Function(PlatformInt64 field0)? integer,
    TResult? Function(double field0)? real,
    TResult? Function(String field0)? string,
    TResult? Function(String field0)? enum_,
    TResult? Function(bool field0)? boolean,
    TResult? Function(int field0)? entityRef,
    TResult? Function(List<IfcValue> field0)? list,
  }) {
    final _that = this;
    switch (_that) {
      case IfcValue_Null() when null_ != null:
        return null_();
      case IfcValue_Integer() when integer != null:
        return integer(_that.field0);
      case IfcValue_Real() when real != null:
        return real(_that.field0);
      case IfcValue_String() when string != null:
        return string(_that.field0);
      case IfcValue_Enum() when enum_ != null:
        return enum_(_that.field0);
      case IfcValue_Boolean() when boolean != null:
        return boolean(_that.field0);
      case IfcValue_EntityRef() when entityRef != null:
        return entityRef(_that.field0);
      case IfcValue_List() when list != null:
        return list(_that.field0);
      case _:
        return null;
    }
  }
}

/// @nodoc

class IfcValue_Null extends IfcValue {
  const IfcValue_Null() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType && other is IfcValue_Null);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'IfcValue.null_()';
  }
}

/// @nodoc

class IfcValue_Integer extends IfcValue {
  const IfcValue_Integer(this.field0) : super._();

  final PlatformInt64 field0;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $IfcValue_IntegerCopyWith<IfcValue_Integer> get copyWith =>
      _$IfcValue_IntegerCopyWithImpl<IfcValue_Integer>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is IfcValue_Integer &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'IfcValue.integer(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $IfcValue_IntegerCopyWith<$Res> implements $IfcValueCopyWith<$Res> {
  factory $IfcValue_IntegerCopyWith(IfcValue_Integer value, $Res Function(IfcValue_Integer) _then) =
      _$IfcValue_IntegerCopyWithImpl;
  @useResult
  $Res call({PlatformInt64 field0});
}

/// @nodoc
class _$IfcValue_IntegerCopyWithImpl<$Res> implements $IfcValue_IntegerCopyWith<$Res> {
  _$IfcValue_IntegerCopyWithImpl(this._self, this._then);

  final IfcValue_Integer _self;
  final $Res Function(IfcValue_Integer) _then;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(IfcValue_Integer(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
    ));
  }
}

/// @nodoc

class IfcValue_Real extends IfcValue {
  const IfcValue_Real(this.field0) : super._();

  final double field0;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $IfcValue_RealCopyWith<IfcValue_Real> get copyWith => _$IfcValue_RealCopyWithImpl<IfcValue_Real>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is IfcValue_Real &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'IfcValue.real(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $IfcValue_RealCopyWith<$Res> implements $IfcValueCopyWith<$Res> {
  factory $IfcValue_RealCopyWith(IfcValue_Real value, $Res Function(IfcValue_Real) _then) = _$IfcValue_RealCopyWithImpl;
  @useResult
  $Res call({double field0});
}

/// @nodoc
class _$IfcValue_RealCopyWithImpl<$Res> implements $IfcValue_RealCopyWith<$Res> {
  _$IfcValue_RealCopyWithImpl(this._self, this._then);

  final IfcValue_Real _self;
  final $Res Function(IfcValue_Real) _then;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(IfcValue_Real(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as double,
    ));
  }
}

/// @nodoc

class IfcValue_String extends IfcValue {
  const IfcValue_String(this.field0) : super._();

  final String field0;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $IfcValue_StringCopyWith<IfcValue_String> get copyWith =>
      _$IfcValue_StringCopyWithImpl<IfcValue_String>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is IfcValue_String &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'IfcValue.string(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $IfcValue_StringCopyWith<$Res> implements $IfcValueCopyWith<$Res> {
  factory $IfcValue_StringCopyWith(IfcValue_String value, $Res Function(IfcValue_String) _then) =
      _$IfcValue_StringCopyWithImpl;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class _$IfcValue_StringCopyWithImpl<$Res> implements $IfcValue_StringCopyWith<$Res> {
  _$IfcValue_StringCopyWithImpl(this._self, this._then);

  final IfcValue_String _self;
  final $Res Function(IfcValue_String) _then;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(IfcValue_String(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class IfcValue_Enum extends IfcValue {
  const IfcValue_Enum(this.field0) : super._();

  final String field0;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $IfcValue_EnumCopyWith<IfcValue_Enum> get copyWith => _$IfcValue_EnumCopyWithImpl<IfcValue_Enum>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is IfcValue_Enum &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'IfcValue.enum_(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $IfcValue_EnumCopyWith<$Res> implements $IfcValueCopyWith<$Res> {
  factory $IfcValue_EnumCopyWith(IfcValue_Enum value, $Res Function(IfcValue_Enum) _then) = _$IfcValue_EnumCopyWithImpl;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class _$IfcValue_EnumCopyWithImpl<$Res> implements $IfcValue_EnumCopyWith<$Res> {
  _$IfcValue_EnumCopyWithImpl(this._self, this._then);

  final IfcValue_Enum _self;
  final $Res Function(IfcValue_Enum) _then;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(IfcValue_Enum(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class IfcValue_Boolean extends IfcValue {
  const IfcValue_Boolean(this.field0) : super._();

  final bool field0;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $IfcValue_BooleanCopyWith<IfcValue_Boolean> get copyWith =>
      _$IfcValue_BooleanCopyWithImpl<IfcValue_Boolean>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is IfcValue_Boolean &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'IfcValue.boolean(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $IfcValue_BooleanCopyWith<$Res> implements $IfcValueCopyWith<$Res> {
  factory $IfcValue_BooleanCopyWith(IfcValue_Boolean value, $Res Function(IfcValue_Boolean) _then) =
      _$IfcValue_BooleanCopyWithImpl;
  @useResult
  $Res call({bool field0});
}

/// @nodoc
class _$IfcValue_BooleanCopyWithImpl<$Res> implements $IfcValue_BooleanCopyWith<$Res> {
  _$IfcValue_BooleanCopyWithImpl(this._self, this._then);

  final IfcValue_Boolean _self;
  final $Res Function(IfcValue_Boolean) _then;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(IfcValue_Boolean(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as bool,
    ));
  }
}

/// @nodoc

class IfcValue_EntityRef extends IfcValue {
  const IfcValue_EntityRef(this.field0) : super._();

  final int field0;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $IfcValue_EntityRefCopyWith<IfcValue_EntityRef> get copyWith =>
      _$IfcValue_EntityRefCopyWithImpl<IfcValue_EntityRef>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is IfcValue_EntityRef &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'IfcValue.entityRef(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $IfcValue_EntityRefCopyWith<$Res> implements $IfcValueCopyWith<$Res> {
  factory $IfcValue_EntityRefCopyWith(IfcValue_EntityRef value, $Res Function(IfcValue_EntityRef) _then) =
      _$IfcValue_EntityRefCopyWithImpl;
  @useResult
  $Res call({int field0});
}

/// @nodoc
class _$IfcValue_EntityRefCopyWithImpl<$Res> implements $IfcValue_EntityRefCopyWith<$Res> {
  _$IfcValue_EntityRefCopyWithImpl(this._self, this._then);

  final IfcValue_EntityRef _self;
  final $Res Function(IfcValue_EntityRef) _then;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(IfcValue_EntityRef(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as int,
    ));
  }
}

/// @nodoc

class IfcValue_List extends IfcValue {
  const IfcValue_List(final List<IfcValue> field0)
      : _field0 = field0,
        super._();

  final List<IfcValue> _field0;
  List<IfcValue> get field0 {
    if (_field0 is EqualUnmodifiableListView) return _field0;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_field0);
  }

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $IfcValue_ListCopyWith<IfcValue_List> get copyWith => _$IfcValue_ListCopyWithImpl<IfcValue_List>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is IfcValue_List &&
            const DeepCollectionEquality().equals(other._field0, _field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, const DeepCollectionEquality().hash(_field0));

  @override
  String toString() {
    return 'IfcValue.list(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $IfcValue_ListCopyWith<$Res> implements $IfcValueCopyWith<$Res> {
  factory $IfcValue_ListCopyWith(IfcValue_List value, $Res Function(IfcValue_List) _then) = _$IfcValue_ListCopyWithImpl;
  @useResult
  $Res call({List<IfcValue> field0});
}

/// @nodoc
class _$IfcValue_ListCopyWithImpl<$Res> implements $IfcValue_ListCopyWith<$Res> {
  _$IfcValue_ListCopyWithImpl(this._self, this._then);

  final IfcValue_List _self;
  final $Res Function(IfcValue_List) _then;

  /// Create a copy of IfcValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(IfcValue_List(
      null == field0
          ? _self._field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as List<IfcValue>,
    ));
  }
}

// dart format on
