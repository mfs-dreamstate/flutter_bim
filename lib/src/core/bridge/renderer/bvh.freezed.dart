// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'bvh.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$BvhNode {
  F32Array3 get min;
  F32Array3 get max;

  /// Create a copy of BvhNode
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $BvhNodeCopyWith<BvhNode> get copyWith => _$BvhNodeCopyWithImpl<BvhNode>(this as BvhNode, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is BvhNode &&
            const DeepCollectionEquality().equals(other.min, min) &&
            const DeepCollectionEquality().equals(other.max, max));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(min), const DeepCollectionEquality().hash(max));

  @override
  String toString() {
    return 'BvhNode(min: $min, max: $max)';
  }
}

/// @nodoc
abstract mixin class $BvhNodeCopyWith<$Res> {
  factory $BvhNodeCopyWith(BvhNode value, $Res Function(BvhNode) _then) = _$BvhNodeCopyWithImpl;
  @useResult
  $Res call({F32Array3 min, F32Array3 max});
}

/// @nodoc
class _$BvhNodeCopyWithImpl<$Res> implements $BvhNodeCopyWith<$Res> {
  _$BvhNodeCopyWithImpl(this._self, this._then);

  final BvhNode _self;
  final $Res Function(BvhNode) _then;

  /// Create a copy of BvhNode
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? min = null,
    Object? max = null,
  }) {
    return _then(_self.copyWith(
      min: null == min
          ? _self.min
          : min // ignore: cast_nullable_to_non_nullable
              as F32Array3,
      max: null == max
          ? _self.max
          : max // ignore: cast_nullable_to_non_nullable
              as F32Array3,
    ));
  }
}

/// Adds pattern-matching-related methods to [BvhNode].
extension BvhNodePatterns on BvhNode {
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
    TResult Function(BvhNode_Leaf value)? leaf,
    TResult Function(BvhNode_Internal value)? internal,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case BvhNode_Leaf() when leaf != null:
        return leaf(_that);
      case BvhNode_Internal() when internal != null:
        return internal(_that);
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
    required TResult Function(BvhNode_Leaf value) leaf,
    required TResult Function(BvhNode_Internal value) internal,
  }) {
    final _that = this;
    switch (_that) {
      case BvhNode_Leaf():
        return leaf(_that);
      case BvhNode_Internal():
        return internal(_that);
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
    TResult? Function(BvhNode_Leaf value)? leaf,
    TResult? Function(BvhNode_Internal value)? internal,
  }) {
    final _that = this;
    switch (_that) {
      case BvhNode_Leaf() when leaf != null:
        return leaf(_that);
      case BvhNode_Internal() when internal != null:
        return internal(_that);
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
    TResult Function(F32Array3 min, F32Array3 max, BigInt elementIndex)? leaf,
    TResult Function(F32Array3 min, F32Array3 max, BvhNode left, BvhNode right)? internal,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case BvhNode_Leaf() when leaf != null:
        return leaf(_that.min, _that.max, _that.elementIndex);
      case BvhNode_Internal() when internal != null:
        return internal(_that.min, _that.max, _that.left, _that.right);
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
    required TResult Function(F32Array3 min, F32Array3 max, BigInt elementIndex) leaf,
    required TResult Function(F32Array3 min, F32Array3 max, BvhNode left, BvhNode right) internal,
  }) {
    final _that = this;
    switch (_that) {
      case BvhNode_Leaf():
        return leaf(_that.min, _that.max, _that.elementIndex);
      case BvhNode_Internal():
        return internal(_that.min, _that.max, _that.left, _that.right);
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
    TResult? Function(F32Array3 min, F32Array3 max, BigInt elementIndex)? leaf,
    TResult? Function(F32Array3 min, F32Array3 max, BvhNode left, BvhNode right)? internal,
  }) {
    final _that = this;
    switch (_that) {
      case BvhNode_Leaf() when leaf != null:
        return leaf(_that.min, _that.max, _that.elementIndex);
      case BvhNode_Internal() when internal != null:
        return internal(_that.min, _that.max, _that.left, _that.right);
      case _:
        return null;
    }
  }
}

/// @nodoc

class BvhNode_Leaf extends BvhNode {
  const BvhNode_Leaf({required this.min, required this.max, required this.elementIndex}) : super._();

  @override
  final F32Array3 min;
  @override
  final F32Array3 max;
  final BigInt elementIndex;

  /// Create a copy of BvhNode
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $BvhNode_LeafCopyWith<BvhNode_Leaf> get copyWith => _$BvhNode_LeafCopyWithImpl<BvhNode_Leaf>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is BvhNode_Leaf &&
            const DeepCollectionEquality().equals(other.min, min) &&
            const DeepCollectionEquality().equals(other.max, max) &&
            (identical(other.elementIndex, elementIndex) || other.elementIndex == elementIndex));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType, const DeepCollectionEquality().hash(min), const DeepCollectionEquality().hash(max), elementIndex);

  @override
  String toString() {
    return 'BvhNode.leaf(min: $min, max: $max, elementIndex: $elementIndex)';
  }
}

/// @nodoc
abstract mixin class $BvhNode_LeafCopyWith<$Res> implements $BvhNodeCopyWith<$Res> {
  factory $BvhNode_LeafCopyWith(BvhNode_Leaf value, $Res Function(BvhNode_Leaf) _then) = _$BvhNode_LeafCopyWithImpl;
  @override
  @useResult
  $Res call({F32Array3 min, F32Array3 max, BigInt elementIndex});
}

/// @nodoc
class _$BvhNode_LeafCopyWithImpl<$Res> implements $BvhNode_LeafCopyWith<$Res> {
  _$BvhNode_LeafCopyWithImpl(this._self, this._then);

  final BvhNode_Leaf _self;
  final $Res Function(BvhNode_Leaf) _then;

  /// Create a copy of BvhNode
  /// with the given fields replaced by the non-null parameter values.
  @override
  @pragma('vm:prefer-inline')
  $Res call({
    Object? min = null,
    Object? max = null,
    Object? elementIndex = null,
  }) {
    return _then(BvhNode_Leaf(
      min: null == min
          ? _self.min
          : min // ignore: cast_nullable_to_non_nullable
              as F32Array3,
      max: null == max
          ? _self.max
          : max // ignore: cast_nullable_to_non_nullable
              as F32Array3,
      elementIndex: null == elementIndex
          ? _self.elementIndex
          : elementIndex // ignore: cast_nullable_to_non_nullable
              as BigInt,
    ));
  }
}

/// @nodoc

class BvhNode_Internal extends BvhNode {
  const BvhNode_Internal({required this.min, required this.max, required this.left, required this.right}) : super._();

  @override
  final F32Array3 min;
  @override
  final F32Array3 max;
  final BvhNode left;
  final BvhNode right;

  /// Create a copy of BvhNode
  /// with the given fields replaced by the non-null parameter values.
  @override
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $BvhNode_InternalCopyWith<BvhNode_Internal> get copyWith =>
      _$BvhNode_InternalCopyWithImpl<BvhNode_Internal>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is BvhNode_Internal &&
            const DeepCollectionEquality().equals(other.min, min) &&
            const DeepCollectionEquality().equals(other.max, max) &&
            (identical(other.left, left) || other.left == left) &&
            (identical(other.right, right) || other.right == right));
  }

  @override
  int get hashCode => Object.hash(
      runtimeType, const DeepCollectionEquality().hash(min), const DeepCollectionEquality().hash(max), left, right);

  @override
  String toString() {
    return 'BvhNode.internal(min: $min, max: $max, left: $left, right: $right)';
  }
}

/// @nodoc
abstract mixin class $BvhNode_InternalCopyWith<$Res> implements $BvhNodeCopyWith<$Res> {
  factory $BvhNode_InternalCopyWith(BvhNode_Internal value, $Res Function(BvhNode_Internal) _then) =
      _$BvhNode_InternalCopyWithImpl;
  @override
  @useResult
  $Res call({F32Array3 min, F32Array3 max, BvhNode left, BvhNode right});

  $BvhNodeCopyWith<$Res> get left;
  $BvhNodeCopyWith<$Res> get right;
}

/// @nodoc
class _$BvhNode_InternalCopyWithImpl<$Res> implements $BvhNode_InternalCopyWith<$Res> {
  _$BvhNode_InternalCopyWithImpl(this._self, this._then);

  final BvhNode_Internal _self;
  final $Res Function(BvhNode_Internal) _then;

  /// Create a copy of BvhNode
  /// with the given fields replaced by the non-null parameter values.
  @override
  @pragma('vm:prefer-inline')
  $Res call({
    Object? min = null,
    Object? max = null,
    Object? left = null,
    Object? right = null,
  }) {
    return _then(BvhNode_Internal(
      min: null == min
          ? _self.min
          : min // ignore: cast_nullable_to_non_nullable
              as F32Array3,
      max: null == max
          ? _self.max
          : max // ignore: cast_nullable_to_non_nullable
              as F32Array3,
      left: null == left
          ? _self.left
          : left // ignore: cast_nullable_to_non_nullable
              as BvhNode,
      right: null == right
          ? _self.right
          : right // ignore: cast_nullable_to_non_nullable
              as BvhNode,
    ));
  }

  /// Create a copy of BvhNode
  /// with the given fields replaced by the non-null parameter values.
  @override
  @pragma('vm:prefer-inline')
  $BvhNodeCopyWith<$Res> get left {
    return $BvhNodeCopyWith<$Res>(_self.left, (value) {
      return _then(_self.copyWith(left: value));
    });
  }

  /// Create a copy of BvhNode
  /// with the given fields replaced by the non-null parameter values.
  @override
  @pragma('vm:prefer-inline')
  $BvhNodeCopyWith<$Res> get right {
    return $BvhNodeCopyWith<$Res>(_self.right, (value) {
      return _then(_self.copyWith(right: value));
    });
  }
}

// dart format on
