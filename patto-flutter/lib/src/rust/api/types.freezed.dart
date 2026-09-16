// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$BlockKind {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is BlockKind);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'BlockKind()';
}


}

/// @nodoc
class $BlockKindCopyWith<$Res>  {
$BlockKindCopyWith(BlockKind _, $Res Function(BlockKind) __);
}


/// Adds pattern-matching-related methods to [BlockKind].
extension BlockKindPatterns on BlockKind {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( BlockKind_Line value)?  line,TResult Function( BlockKind_Blank value)?  blank,TResult Function( BlockKind_Code value)?  code,TResult Function( BlockKind_Math value)?  math,TResult Function( BlockKind_Table value)?  table,TResult Function( BlockKind_Images value)?  images,TResult Function( BlockKind_Rule value)?  rule,required TResult orElse(),}){
final _that = this;
switch (_that) {
case BlockKind_Line() when line != null:
return line(_that);case BlockKind_Blank() when blank != null:
return blank(_that);case BlockKind_Code() when code != null:
return code(_that);case BlockKind_Math() when math != null:
return math(_that);case BlockKind_Table() when table != null:
return table(_that);case BlockKind_Images() when images != null:
return images(_that);case BlockKind_Rule() when rule != null:
return rule(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( BlockKind_Line value)  line,required TResult Function( BlockKind_Blank value)  blank,required TResult Function( BlockKind_Code value)  code,required TResult Function( BlockKind_Math value)  math,required TResult Function( BlockKind_Table value)  table,required TResult Function( BlockKind_Images value)  images,required TResult Function( BlockKind_Rule value)  rule,}){
final _that = this;
switch (_that) {
case BlockKind_Line():
return line(_that);case BlockKind_Blank():
return blank(_that);case BlockKind_Code():
return code(_that);case BlockKind_Math():
return math(_that);case BlockKind_Table():
return table(_that);case BlockKind_Images():
return images(_that);case BlockKind_Rule():
return rule(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( BlockKind_Line value)?  line,TResult? Function( BlockKind_Blank value)?  blank,TResult? Function( BlockKind_Code value)?  code,TResult? Function( BlockKind_Math value)?  math,TResult? Function( BlockKind_Table value)?  table,TResult? Function( BlockKind_Images value)?  images,TResult? Function( BlockKind_Rule value)?  rule,}){
final _that = this;
switch (_that) {
case BlockKind_Line() when line != null:
return line(_that);case BlockKind_Blank() when blank != null:
return blank(_that);case BlockKind_Code() when code != null:
return code(_that);case BlockKind_Math() when math != null:
return math(_that);case BlockKind_Table() when table != null:
return table(_that);case BlockKind_Images() when images != null:
return images(_that);case BlockKind_Rule() when rule != null:
return rule(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( List<NoteSpan> spans)?  line,TResult Function()?  blank,TResult Function( String lang,  List<String> lines)?  code,TResult Function( String tex)?  math,TResult Function( String? caption,  List<NoteTableRow> rows)?  table,TResult Function( List<ImageRef> images)?  images,TResult Function()?  rule,required TResult orElse(),}) {final _that = this;
switch (_that) {
case BlockKind_Line() when line != null:
return line(_that.spans);case BlockKind_Blank() when blank != null:
return blank();case BlockKind_Code() when code != null:
return code(_that.lang,_that.lines);case BlockKind_Math() when math != null:
return math(_that.tex);case BlockKind_Table() when table != null:
return table(_that.caption,_that.rows);case BlockKind_Images() when images != null:
return images(_that.images);case BlockKind_Rule() when rule != null:
return rule();case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( List<NoteSpan> spans)  line,required TResult Function()  blank,required TResult Function( String lang,  List<String> lines)  code,required TResult Function( String tex)  math,required TResult Function( String? caption,  List<NoteTableRow> rows)  table,required TResult Function( List<ImageRef> images)  images,required TResult Function()  rule,}) {final _that = this;
switch (_that) {
case BlockKind_Line():
return line(_that.spans);case BlockKind_Blank():
return blank();case BlockKind_Code():
return code(_that.lang,_that.lines);case BlockKind_Math():
return math(_that.tex);case BlockKind_Table():
return table(_that.caption,_that.rows);case BlockKind_Images():
return images(_that.images);case BlockKind_Rule():
return rule();}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( List<NoteSpan> spans)?  line,TResult? Function()?  blank,TResult? Function( String lang,  List<String> lines)?  code,TResult? Function( String tex)?  math,TResult? Function( String? caption,  List<NoteTableRow> rows)?  table,TResult? Function( List<ImageRef> images)?  images,TResult? Function()?  rule,}) {final _that = this;
switch (_that) {
case BlockKind_Line() when line != null:
return line(_that.spans);case BlockKind_Blank() when blank != null:
return blank();case BlockKind_Code() when code != null:
return code(_that.lang,_that.lines);case BlockKind_Math() when math != null:
return math(_that.tex);case BlockKind_Table() when table != null:
return table(_that.caption,_that.rows);case BlockKind_Images() when images != null:
return images(_that.images);case BlockKind_Rule() when rule != null:
return rule();case _:
  return null;

}
}

}

/// @nodoc


class BlockKind_Line extends BlockKind {
  const BlockKind_Line({required  List<NoteSpan> spans}): _spans = spans,super._();
  

 final  List<NoteSpan> _spans;
 List<NoteSpan> get spans {
  if (_spans is EqualUnmodifiableListView) return _spans;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_spans);
}


/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$BlockKind_LineCopyWith<BlockKind_Line> get copyWith => _$BlockKind_LineCopyWithImpl<BlockKind_Line>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is BlockKind_Line&&const DeepCollectionEquality().equals(other.spans, _spans));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_spans));
}

@override
String toString() {
    return 'BlockKind.line(spans: $spans)';
}


}

/// @nodoc
abstract mixin class $BlockKind_LineCopyWith<$Res> implements $BlockKindCopyWith<$Res> {
  factory $BlockKind_LineCopyWith(BlockKind_Line value, $Res Function(BlockKind_Line) _then) = _$BlockKind_LineCopyWithImpl;
@useResult
$Res call({
 List<NoteSpan> spans
});




}
/// @nodoc
class _$BlockKind_LineCopyWithImpl<$Res>
    implements $BlockKind_LineCopyWith<$Res> {
  _$BlockKind_LineCopyWithImpl(this._self, this._then);

  final BlockKind_Line _self;
  final $Res Function(BlockKind_Line) _then;

/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? spans = null,}) {
  return _then(BlockKind_Line(
spans: null == spans ? _self._spans : spans // ignore: cast_nullable_to_non_nullable
as List<NoteSpan>,
  ));
}


}

/// @nodoc


class BlockKind_Blank extends BlockKind {
  const BlockKind_Blank(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is BlockKind_Blank);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'BlockKind.blank()';
}


}




/// @nodoc


class BlockKind_Code extends BlockKind {
  const BlockKind_Code({required this.lang, required  List<String> lines}): _lines = lines,super._();
  

 final  String lang;
 final  List<String> _lines;
 List<String> get lines {
  if (_lines is EqualUnmodifiableListView) return _lines;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_lines);
}


/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$BlockKind_CodeCopyWith<BlockKind_Code> get copyWith => _$BlockKind_CodeCopyWithImpl<BlockKind_Code>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is BlockKind_Code&&(identical(other.lang, lang) || other.lang == lang)&&const DeepCollectionEquality().equals(other.lines, _lines));
}


@override
int get hashCode {
    return Object.hash(runtimeType,lang,const DeepCollectionEquality().hash(_lines));
}

@override
String toString() {
    return 'BlockKind.code(lang: $lang, lines: $lines)';
}


}

/// @nodoc
abstract mixin class $BlockKind_CodeCopyWith<$Res> implements $BlockKindCopyWith<$Res> {
  factory $BlockKind_CodeCopyWith(BlockKind_Code value, $Res Function(BlockKind_Code) _then) = _$BlockKind_CodeCopyWithImpl;
@useResult
$Res call({
 String lang, List<String> lines
});




}
/// @nodoc
class _$BlockKind_CodeCopyWithImpl<$Res>
    implements $BlockKind_CodeCopyWith<$Res> {
  _$BlockKind_CodeCopyWithImpl(this._self, this._then);

  final BlockKind_Code _self;
  final $Res Function(BlockKind_Code) _then;

/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lang = null,Object? lines = null,}) {
  return _then(BlockKind_Code(
lang: null == lang ? _self.lang : lang // ignore: cast_nullable_to_non_nullable
as String,lines: null == lines ? _self._lines : lines // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class BlockKind_Math extends BlockKind {
  const BlockKind_Math({required this.tex}): super._();
  

 final  String tex;

/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$BlockKind_MathCopyWith<BlockKind_Math> get copyWith => _$BlockKind_MathCopyWithImpl<BlockKind_Math>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is BlockKind_Math&&(identical(other.tex, tex) || other.tex == tex));
}


@override
int get hashCode {
    return Object.hash(runtimeType,tex);
}

@override
String toString() {
    return 'BlockKind.math(tex: $tex)';
}


}

/// @nodoc
abstract mixin class $BlockKind_MathCopyWith<$Res> implements $BlockKindCopyWith<$Res> {
  factory $BlockKind_MathCopyWith(BlockKind_Math value, $Res Function(BlockKind_Math) _then) = _$BlockKind_MathCopyWithImpl;
@useResult
$Res call({
 String tex
});




}
/// @nodoc
class _$BlockKind_MathCopyWithImpl<$Res>
    implements $BlockKind_MathCopyWith<$Res> {
  _$BlockKind_MathCopyWithImpl(this._self, this._then);

  final BlockKind_Math _self;
  final $Res Function(BlockKind_Math) _then;

/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? tex = null,}) {
  return _then(BlockKind_Math(
tex: null == tex ? _self.tex : tex // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class BlockKind_Table extends BlockKind {
  const BlockKind_Table({this.caption, required  List<NoteTableRow> rows}): _rows = rows,super._();
  

 final  String? caption;
 final  List<NoteTableRow> _rows;
 List<NoteTableRow> get rows {
  if (_rows is EqualUnmodifiableListView) return _rows;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_rows);
}


/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$BlockKind_TableCopyWith<BlockKind_Table> get copyWith => _$BlockKind_TableCopyWithImpl<BlockKind_Table>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is BlockKind_Table&&(identical(other.caption, caption) || other.caption == caption)&&const DeepCollectionEquality().equals(other.rows, _rows));
}


@override
int get hashCode {
    return Object.hash(runtimeType,caption,const DeepCollectionEquality().hash(_rows));
}

@override
String toString() {
    return 'BlockKind.table(caption: $caption, rows: $rows)';
}


}

/// @nodoc
abstract mixin class $BlockKind_TableCopyWith<$Res> implements $BlockKindCopyWith<$Res> {
  factory $BlockKind_TableCopyWith(BlockKind_Table value, $Res Function(BlockKind_Table) _then) = _$BlockKind_TableCopyWithImpl;
@useResult
$Res call({
 String? caption, List<NoteTableRow> rows
});




}
/// @nodoc
class _$BlockKind_TableCopyWithImpl<$Res>
    implements $BlockKind_TableCopyWith<$Res> {
  _$BlockKind_TableCopyWithImpl(this._self, this._then);

  final BlockKind_Table _self;
  final $Res Function(BlockKind_Table) _then;

/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? caption = freezed,Object? rows = null,}) {
  return _then(BlockKind_Table(
caption: freezed == caption ? _self.caption : caption // ignore: cast_nullable_to_non_nullable
as String?,rows: null == rows ? _self._rows : rows // ignore: cast_nullable_to_non_nullable
as List<NoteTableRow>,
  ));
}


}

/// @nodoc


class BlockKind_Images extends BlockKind {
  const BlockKind_Images({required  List<ImageRef> images}): _images = images,super._();
  

 final  List<ImageRef> _images;
 List<ImageRef> get images {
  if (_images is EqualUnmodifiableListView) return _images;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_images);
}


/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$BlockKind_ImagesCopyWith<BlockKind_Images> get copyWith => _$BlockKind_ImagesCopyWithImpl<BlockKind_Images>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is BlockKind_Images&&const DeepCollectionEquality().equals(other.images, _images));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_images));
}

@override
String toString() {
    return 'BlockKind.images(images: $images)';
}


}

/// @nodoc
abstract mixin class $BlockKind_ImagesCopyWith<$Res> implements $BlockKindCopyWith<$Res> {
  factory $BlockKind_ImagesCopyWith(BlockKind_Images value, $Res Function(BlockKind_Images) _then) = _$BlockKind_ImagesCopyWithImpl;
@useResult
$Res call({
 List<ImageRef> images
});




}
/// @nodoc
class _$BlockKind_ImagesCopyWithImpl<$Res>
    implements $BlockKind_ImagesCopyWith<$Res> {
  _$BlockKind_ImagesCopyWithImpl(this._self, this._then);

  final BlockKind_Images _self;
  final $Res Function(BlockKind_Images) _then;

/// Create a copy of BlockKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? images = null,}) {
  return _then(BlockKind_Images(
images: null == images ? _self._images : images // ignore: cast_nullable_to_non_nullable
as List<ImageRef>,
  ));
}


}

/// @nodoc


class BlockKind_Rule extends BlockKind {
  const BlockKind_Rule(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is BlockKind_Rule);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'BlockKind.rule()';
}


}




/// @nodoc
mixin _$EmbedKind {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbedKind);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EmbedKind()';
}


}

/// @nodoc
class $EmbedKindCopyWith<$Res>  {
$EmbedKindCopyWith(EmbedKind _, $Res Function(EmbedKind) __);
}


/// Adds pattern-matching-related methods to [EmbedKind].
extension EmbedKindPatterns on EmbedKind {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( EmbedKind_Youtube value)?  youtube,TResult Function( EmbedKind_Twitter value)?  twitter,TResult Function( EmbedKind_SpeakerDeck value)?  speakerDeck,TResult Function( EmbedKind_SlideShare value)?  slideShare,TResult Function( EmbedKind_Pdf value)?  pdf,TResult Function( EmbedKind_Other value)?  other,required TResult orElse(),}){
final _that = this;
switch (_that) {
case EmbedKind_Youtube() when youtube != null:
return youtube(_that);case EmbedKind_Twitter() when twitter != null:
return twitter(_that);case EmbedKind_SpeakerDeck() when speakerDeck != null:
return speakerDeck(_that);case EmbedKind_SlideShare() when slideShare != null:
return slideShare(_that);case EmbedKind_Pdf() when pdf != null:
return pdf(_that);case EmbedKind_Other() when other != null:
return other(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( EmbedKind_Youtube value)  youtube,required TResult Function( EmbedKind_Twitter value)  twitter,required TResult Function( EmbedKind_SpeakerDeck value)  speakerDeck,required TResult Function( EmbedKind_SlideShare value)  slideShare,required TResult Function( EmbedKind_Pdf value)  pdf,required TResult Function( EmbedKind_Other value)  other,}){
final _that = this;
switch (_that) {
case EmbedKind_Youtube():
return youtube(_that);case EmbedKind_Twitter():
return twitter(_that);case EmbedKind_SpeakerDeck():
return speakerDeck(_that);case EmbedKind_SlideShare():
return slideShare(_that);case EmbedKind_Pdf():
return pdf(_that);case EmbedKind_Other():
return other(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( EmbedKind_Youtube value)?  youtube,TResult? Function( EmbedKind_Twitter value)?  twitter,TResult? Function( EmbedKind_SpeakerDeck value)?  speakerDeck,TResult? Function( EmbedKind_SlideShare value)?  slideShare,TResult? Function( EmbedKind_Pdf value)?  pdf,TResult? Function( EmbedKind_Other value)?  other,}){
final _that = this;
switch (_that) {
case EmbedKind_Youtube() when youtube != null:
return youtube(_that);case EmbedKind_Twitter() when twitter != null:
return twitter(_that);case EmbedKind_SpeakerDeck() when speakerDeck != null:
return speakerDeck(_that);case EmbedKind_SlideShare() when slideShare != null:
return slideShare(_that);case EmbedKind_Pdf() when pdf != null:
return pdf(_that);case EmbedKind_Other() when other != null:
return other(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String videoId)?  youtube,TResult Function()?  twitter,TResult Function()?  speakerDeck,TResult Function()?  slideShare,TResult Function()?  pdf,TResult Function()?  other,required TResult orElse(),}) {final _that = this;
switch (_that) {
case EmbedKind_Youtube() when youtube != null:
return youtube(_that.videoId);case EmbedKind_Twitter() when twitter != null:
return twitter();case EmbedKind_SpeakerDeck() when speakerDeck != null:
return speakerDeck();case EmbedKind_SlideShare() when slideShare != null:
return slideShare();case EmbedKind_Pdf() when pdf != null:
return pdf();case EmbedKind_Other() when other != null:
return other();case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String videoId)  youtube,required TResult Function()  twitter,required TResult Function()  speakerDeck,required TResult Function()  slideShare,required TResult Function()  pdf,required TResult Function()  other,}) {final _that = this;
switch (_that) {
case EmbedKind_Youtube():
return youtube(_that.videoId);case EmbedKind_Twitter():
return twitter();case EmbedKind_SpeakerDeck():
return speakerDeck();case EmbedKind_SlideShare():
return slideShare();case EmbedKind_Pdf():
return pdf();case EmbedKind_Other():
return other();}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String videoId)?  youtube,TResult? Function()?  twitter,TResult? Function()?  speakerDeck,TResult? Function()?  slideShare,TResult? Function()?  pdf,TResult? Function()?  other,}) {final _that = this;
switch (_that) {
case EmbedKind_Youtube() when youtube != null:
return youtube(_that.videoId);case EmbedKind_Twitter() when twitter != null:
return twitter();case EmbedKind_SpeakerDeck() when speakerDeck != null:
return speakerDeck();case EmbedKind_SlideShare() when slideShare != null:
return slideShare();case EmbedKind_Pdf() when pdf != null:
return pdf();case EmbedKind_Other() when other != null:
return other();case _:
  return null;

}
}

}

/// @nodoc


class EmbedKind_Youtube extends EmbedKind {
  const EmbedKind_Youtube({required this.videoId}): super._();
  

 final  String videoId;

/// Create a copy of EmbedKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$EmbedKind_YoutubeCopyWith<EmbedKind_Youtube> get copyWith => _$EmbedKind_YoutubeCopyWithImpl<EmbedKind_Youtube>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbedKind_Youtube&&(identical(other.videoId, videoId) || other.videoId == videoId));
}


@override
int get hashCode {
    return Object.hash(runtimeType,videoId);
}

@override
String toString() {
    return 'EmbedKind.youtube(videoId: $videoId)';
}


}

/// @nodoc
abstract mixin class $EmbedKind_YoutubeCopyWith<$Res> implements $EmbedKindCopyWith<$Res> {
  factory $EmbedKind_YoutubeCopyWith(EmbedKind_Youtube value, $Res Function(EmbedKind_Youtube) _then) = _$EmbedKind_YoutubeCopyWithImpl;
@useResult
$Res call({
 String videoId
});




}
/// @nodoc
class _$EmbedKind_YoutubeCopyWithImpl<$Res>
    implements $EmbedKind_YoutubeCopyWith<$Res> {
  _$EmbedKind_YoutubeCopyWithImpl(this._self, this._then);

  final EmbedKind_Youtube _self;
  final $Res Function(EmbedKind_Youtube) _then;

/// Create a copy of EmbedKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? videoId = null,}) {
  return _then(EmbedKind_Youtube(
videoId: null == videoId ? _self.videoId : videoId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class EmbedKind_Twitter extends EmbedKind {
  const EmbedKind_Twitter(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbedKind_Twitter);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EmbedKind.twitter()';
}


}




/// @nodoc


class EmbedKind_SpeakerDeck extends EmbedKind {
  const EmbedKind_SpeakerDeck(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbedKind_SpeakerDeck);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EmbedKind.speakerDeck()';
}


}




/// @nodoc


class EmbedKind_SlideShare extends EmbedKind {
  const EmbedKind_SlideShare(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbedKind_SlideShare);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EmbedKind.slideShare()';
}


}




/// @nodoc


class EmbedKind_Pdf extends EmbedKind {
  const EmbedKind_Pdf(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbedKind_Pdf);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EmbedKind.pdf()';
}


}




/// @nodoc


class EmbedKind_Other extends EmbedKind {
  const EmbedKind_Other(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is EmbedKind_Other);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'EmbedKind.other()';
}


}




/// @nodoc
mixin _$NoteSpan {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is NoteSpan);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'NoteSpan()';
}


}

/// @nodoc
class $NoteSpanCopyWith<$Res>  {
$NoteSpanCopyWith(NoteSpan _, $Res Function(NoteSpan) __);
}


/// Adds pattern-matching-related methods to [NoteSpan].
extension NoteSpanPatterns on NoteSpan {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( NoteSpan_Text value)?  text,TResult Function( NoteSpan_Decoration value)?  decoration,TResult Function( NoteSpan_WikiLink value)?  wikiLink,TResult Function( NoteSpan_Url value)?  url,TResult Function( NoteSpan_InlineCode value)?  inlineCode,TResult Function( NoteSpan_InlineMath value)?  inlineMath,TResult Function( NoteSpan_Image value)?  image,TResult Function( NoteSpan_Embed value)?  embed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case NoteSpan_Text() when text != null:
return text(_that);case NoteSpan_Decoration() when decoration != null:
return decoration(_that);case NoteSpan_WikiLink() when wikiLink != null:
return wikiLink(_that);case NoteSpan_Url() when url != null:
return url(_that);case NoteSpan_InlineCode() when inlineCode != null:
return inlineCode(_that);case NoteSpan_InlineMath() when inlineMath != null:
return inlineMath(_that);case NoteSpan_Image() when image != null:
return image(_that);case NoteSpan_Embed() when embed != null:
return embed(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( NoteSpan_Text value)  text,required TResult Function( NoteSpan_Decoration value)  decoration,required TResult Function( NoteSpan_WikiLink value)  wikiLink,required TResult Function( NoteSpan_Url value)  url,required TResult Function( NoteSpan_InlineCode value)  inlineCode,required TResult Function( NoteSpan_InlineMath value)  inlineMath,required TResult Function( NoteSpan_Image value)  image,required TResult Function( NoteSpan_Embed value)  embed,}){
final _that = this;
switch (_that) {
case NoteSpan_Text():
return text(_that);case NoteSpan_Decoration():
return decoration(_that);case NoteSpan_WikiLink():
return wikiLink(_that);case NoteSpan_Url():
return url(_that);case NoteSpan_InlineCode():
return inlineCode(_that);case NoteSpan_InlineMath():
return inlineMath(_that);case NoteSpan_Image():
return image(_that);case NoteSpan_Embed():
return embed(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( NoteSpan_Text value)?  text,TResult? Function( NoteSpan_Decoration value)?  decoration,TResult? Function( NoteSpan_WikiLink value)?  wikiLink,TResult? Function( NoteSpan_Url value)?  url,TResult? Function( NoteSpan_InlineCode value)?  inlineCode,TResult? Function( NoteSpan_InlineMath value)?  inlineMath,TResult? Function( NoteSpan_Image value)?  image,TResult? Function( NoteSpan_Embed value)?  embed,}){
final _that = this;
switch (_that) {
case NoteSpan_Text() when text != null:
return text(_that);case NoteSpan_Decoration() when decoration != null:
return decoration(_that);case NoteSpan_WikiLink() when wikiLink != null:
return wikiLink(_that);case NoteSpan_Url() when url != null:
return url(_that);case NoteSpan_InlineCode() when inlineCode != null:
return inlineCode(_that);case NoteSpan_InlineMath() when inlineMath != null:
return inlineMath(_that);case NoteSpan_Image() when image != null:
return image(_that);case NoteSpan_Embed() when embed != null:
return embed(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String text)?  text,TResult Function( int fontsize,  bool italic,  bool underline,  bool deleted,  List<NoteSpan> children)?  decoration,TResult Function( String name,  String? anchor)?  wikiLink,TResult Function( String url,  String? title)?  url,TResult Function( String code)?  inlineCode,TResult Function( String tex)?  inlineMath,TResult Function( ImageRef image)?  image,TResult Function( String url,  String? title,  EmbedKind kind)?  embed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case NoteSpan_Text() when text != null:
return text(_that.text);case NoteSpan_Decoration() when decoration != null:
return decoration(_that.fontsize,_that.italic,_that.underline,_that.deleted,_that.children);case NoteSpan_WikiLink() when wikiLink != null:
return wikiLink(_that.name,_that.anchor);case NoteSpan_Url() when url != null:
return url(_that.url,_that.title);case NoteSpan_InlineCode() when inlineCode != null:
return inlineCode(_that.code);case NoteSpan_InlineMath() when inlineMath != null:
return inlineMath(_that.tex);case NoteSpan_Image() when image != null:
return image(_that.image);case NoteSpan_Embed() when embed != null:
return embed(_that.url,_that.title,_that.kind);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String text)  text,required TResult Function( int fontsize,  bool italic,  bool underline,  bool deleted,  List<NoteSpan> children)  decoration,required TResult Function( String name,  String? anchor)  wikiLink,required TResult Function( String url,  String? title)  url,required TResult Function( String code)  inlineCode,required TResult Function( String tex)  inlineMath,required TResult Function( ImageRef image)  image,required TResult Function( String url,  String? title,  EmbedKind kind)  embed,}) {final _that = this;
switch (_that) {
case NoteSpan_Text():
return text(_that.text);case NoteSpan_Decoration():
return decoration(_that.fontsize,_that.italic,_that.underline,_that.deleted,_that.children);case NoteSpan_WikiLink():
return wikiLink(_that.name,_that.anchor);case NoteSpan_Url():
return url(_that.url,_that.title);case NoteSpan_InlineCode():
return inlineCode(_that.code);case NoteSpan_InlineMath():
return inlineMath(_that.tex);case NoteSpan_Image():
return image(_that.image);case NoteSpan_Embed():
return embed(_that.url,_that.title,_that.kind);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String text)?  text,TResult? Function( int fontsize,  bool italic,  bool underline,  bool deleted,  List<NoteSpan> children)?  decoration,TResult? Function( String name,  String? anchor)?  wikiLink,TResult? Function( String url,  String? title)?  url,TResult? Function( String code)?  inlineCode,TResult? Function( String tex)?  inlineMath,TResult? Function( ImageRef image)?  image,TResult? Function( String url,  String? title,  EmbedKind kind)?  embed,}) {final _that = this;
switch (_that) {
case NoteSpan_Text() when text != null:
return text(_that.text);case NoteSpan_Decoration() when decoration != null:
return decoration(_that.fontsize,_that.italic,_that.underline,_that.deleted,_that.children);case NoteSpan_WikiLink() when wikiLink != null:
return wikiLink(_that.name,_that.anchor);case NoteSpan_Url() when url != null:
return url(_that.url,_that.title);case NoteSpan_InlineCode() when inlineCode != null:
return inlineCode(_that.code);case NoteSpan_InlineMath() when inlineMath != null:
return inlineMath(_that.tex);case NoteSpan_Image() when image != null:
return image(_that.image);case NoteSpan_Embed() when embed != null:
return embed(_that.url,_that.title,_that.kind);case _:
  return null;

}
}

}

/// @nodoc


class NoteSpan_Text extends NoteSpan {
  const NoteSpan_Text({required this.text}): super._();
  

 final  String text;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$NoteSpan_TextCopyWith<NoteSpan_Text> get copyWith => _$NoteSpan_TextCopyWithImpl<NoteSpan_Text>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is NoteSpan_Text&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode {
    return Object.hash(runtimeType,text);
}

@override
String toString() {
    return 'NoteSpan.text(text: $text)';
}


}

/// @nodoc
abstract mixin class $NoteSpan_TextCopyWith<$Res> implements $NoteSpanCopyWith<$Res> {
  factory $NoteSpan_TextCopyWith(NoteSpan_Text value, $Res Function(NoteSpan_Text) _then) = _$NoteSpan_TextCopyWithImpl;
@useResult
$Res call({
 String text
});




}
/// @nodoc
class _$NoteSpan_TextCopyWithImpl<$Res>
    implements $NoteSpan_TextCopyWith<$Res> {
  _$NoteSpan_TextCopyWithImpl(this._self, this._then);

  final NoteSpan_Text _self;
  final $Res Function(NoteSpan_Text) _then;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? text = null,}) {
  return _then(NoteSpan_Text(
text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class NoteSpan_Decoration extends NoteSpan {
  const NoteSpan_Decoration({required this.fontsize, required this.italic, required this.underline, required this.deleted, required  List<NoteSpan> children}): _children = children,super._();
  

 final  int fontsize;
 final  bool italic;
 final  bool underline;
 final  bool deleted;
 final  List<NoteSpan> _children;
 List<NoteSpan> get children {
  if (_children is EqualUnmodifiableListView) return _children;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_children);
}


/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$NoteSpan_DecorationCopyWith<NoteSpan_Decoration> get copyWith => _$NoteSpan_DecorationCopyWithImpl<NoteSpan_Decoration>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is NoteSpan_Decoration&&(identical(other.fontsize, fontsize) || other.fontsize == fontsize)&&(identical(other.italic, italic) || other.italic == italic)&&(identical(other.underline, underline) || other.underline == underline)&&(identical(other.deleted, deleted) || other.deleted == deleted)&&const DeepCollectionEquality().equals(other.children, _children));
}


@override
int get hashCode {
    return Object.hash(runtimeType,fontsize,italic,underline,deleted,const DeepCollectionEquality().hash(_children));
}

@override
String toString() {
    return 'NoteSpan.decoration(fontsize: $fontsize, italic: $italic, underline: $underline, deleted: $deleted, children: $children)';
}


}

/// @nodoc
abstract mixin class $NoteSpan_DecorationCopyWith<$Res> implements $NoteSpanCopyWith<$Res> {
  factory $NoteSpan_DecorationCopyWith(NoteSpan_Decoration value, $Res Function(NoteSpan_Decoration) _then) = _$NoteSpan_DecorationCopyWithImpl;
@useResult
$Res call({
 int fontsize, bool italic, bool underline, bool deleted, List<NoteSpan> children
});




}
/// @nodoc
class _$NoteSpan_DecorationCopyWithImpl<$Res>
    implements $NoteSpan_DecorationCopyWith<$Res> {
  _$NoteSpan_DecorationCopyWithImpl(this._self, this._then);

  final NoteSpan_Decoration _self;
  final $Res Function(NoteSpan_Decoration) _then;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? fontsize = null,Object? italic = null,Object? underline = null,Object? deleted = null,Object? children = null,}) {
  return _then(NoteSpan_Decoration(
fontsize: null == fontsize ? _self.fontsize : fontsize // ignore: cast_nullable_to_non_nullable
as int,italic: null == italic ? _self.italic : italic // ignore: cast_nullable_to_non_nullable
as bool,underline: null == underline ? _self.underline : underline // ignore: cast_nullable_to_non_nullable
as bool,deleted: null == deleted ? _self.deleted : deleted // ignore: cast_nullable_to_non_nullable
as bool,children: null == children ? _self._children : children // ignore: cast_nullable_to_non_nullable
as List<NoteSpan>,
  ));
}


}

/// @nodoc


class NoteSpan_WikiLink extends NoteSpan {
  const NoteSpan_WikiLink({required this.name, this.anchor}): super._();
  

 final  String name;
 final  String? anchor;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$NoteSpan_WikiLinkCopyWith<NoteSpan_WikiLink> get copyWith => _$NoteSpan_WikiLinkCopyWithImpl<NoteSpan_WikiLink>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is NoteSpan_WikiLink&&(identical(other.name, name) || other.name == name)&&(identical(other.anchor, anchor) || other.anchor == anchor));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name,anchor);
}

@override
String toString() {
    return 'NoteSpan.wikiLink(name: $name, anchor: $anchor)';
}


}

/// @nodoc
abstract mixin class $NoteSpan_WikiLinkCopyWith<$Res> implements $NoteSpanCopyWith<$Res> {
  factory $NoteSpan_WikiLinkCopyWith(NoteSpan_WikiLink value, $Res Function(NoteSpan_WikiLink) _then) = _$NoteSpan_WikiLinkCopyWithImpl;
@useResult
$Res call({
 String name, String? anchor
});




}
/// @nodoc
class _$NoteSpan_WikiLinkCopyWithImpl<$Res>
    implements $NoteSpan_WikiLinkCopyWith<$Res> {
  _$NoteSpan_WikiLinkCopyWithImpl(this._self, this._then);

  final NoteSpan_WikiLink _self;
  final $Res Function(NoteSpan_WikiLink) _then;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,Object? anchor = freezed,}) {
  return _then(NoteSpan_WikiLink(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,anchor: freezed == anchor ? _self.anchor : anchor // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc


class NoteSpan_Url extends NoteSpan {
  const NoteSpan_Url({required this.url, this.title}): super._();
  

 final  String url;
 final  String? title;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$NoteSpan_UrlCopyWith<NoteSpan_Url> get copyWith => _$NoteSpan_UrlCopyWithImpl<NoteSpan_Url>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is NoteSpan_Url&&(identical(other.url, url) || other.url == url)&&(identical(other.title, title) || other.title == title));
}


@override
int get hashCode {
    return Object.hash(runtimeType,url,title);
}

@override
String toString() {
    return 'NoteSpan.url(url: $url, title: $title)';
}


}

/// @nodoc
abstract mixin class $NoteSpan_UrlCopyWith<$Res> implements $NoteSpanCopyWith<$Res> {
  factory $NoteSpan_UrlCopyWith(NoteSpan_Url value, $Res Function(NoteSpan_Url) _then) = _$NoteSpan_UrlCopyWithImpl;
@useResult
$Res call({
 String url, String? title
});




}
/// @nodoc
class _$NoteSpan_UrlCopyWithImpl<$Res>
    implements $NoteSpan_UrlCopyWith<$Res> {
  _$NoteSpan_UrlCopyWithImpl(this._self, this._then);

  final NoteSpan_Url _self;
  final $Res Function(NoteSpan_Url) _then;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? url = null,Object? title = freezed,}) {
  return _then(NoteSpan_Url(
url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,title: freezed == title ? _self.title : title // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc


class NoteSpan_InlineCode extends NoteSpan {
  const NoteSpan_InlineCode({required this.code}): super._();
  

 final  String code;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$NoteSpan_InlineCodeCopyWith<NoteSpan_InlineCode> get copyWith => _$NoteSpan_InlineCodeCopyWithImpl<NoteSpan_InlineCode>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is NoteSpan_InlineCode&&(identical(other.code, code) || other.code == code));
}


@override
int get hashCode {
    return Object.hash(runtimeType,code);
}

@override
String toString() {
    return 'NoteSpan.inlineCode(code: $code)';
}


}

/// @nodoc
abstract mixin class $NoteSpan_InlineCodeCopyWith<$Res> implements $NoteSpanCopyWith<$Res> {
  factory $NoteSpan_InlineCodeCopyWith(NoteSpan_InlineCode value, $Res Function(NoteSpan_InlineCode) _then) = _$NoteSpan_InlineCodeCopyWithImpl;
@useResult
$Res call({
 String code
});




}
/// @nodoc
class _$NoteSpan_InlineCodeCopyWithImpl<$Res>
    implements $NoteSpan_InlineCodeCopyWith<$Res> {
  _$NoteSpan_InlineCodeCopyWithImpl(this._self, this._then);

  final NoteSpan_InlineCode _self;
  final $Res Function(NoteSpan_InlineCode) _then;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? code = null,}) {
  return _then(NoteSpan_InlineCode(
code: null == code ? _self.code : code // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class NoteSpan_InlineMath extends NoteSpan {
  const NoteSpan_InlineMath({required this.tex}): super._();
  

 final  String tex;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$NoteSpan_InlineMathCopyWith<NoteSpan_InlineMath> get copyWith => _$NoteSpan_InlineMathCopyWithImpl<NoteSpan_InlineMath>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is NoteSpan_InlineMath&&(identical(other.tex, tex) || other.tex == tex));
}


@override
int get hashCode {
    return Object.hash(runtimeType,tex);
}

@override
String toString() {
    return 'NoteSpan.inlineMath(tex: $tex)';
}


}

/// @nodoc
abstract mixin class $NoteSpan_InlineMathCopyWith<$Res> implements $NoteSpanCopyWith<$Res> {
  factory $NoteSpan_InlineMathCopyWith(NoteSpan_InlineMath value, $Res Function(NoteSpan_InlineMath) _then) = _$NoteSpan_InlineMathCopyWithImpl;
@useResult
$Res call({
 String tex
});




}
/// @nodoc
class _$NoteSpan_InlineMathCopyWithImpl<$Res>
    implements $NoteSpan_InlineMathCopyWith<$Res> {
  _$NoteSpan_InlineMathCopyWithImpl(this._self, this._then);

  final NoteSpan_InlineMath _self;
  final $Res Function(NoteSpan_InlineMath) _then;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? tex = null,}) {
  return _then(NoteSpan_InlineMath(
tex: null == tex ? _self.tex : tex // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class NoteSpan_Image extends NoteSpan {
  const NoteSpan_Image({required this.image}): super._();
  

 final  ImageRef image;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$NoteSpan_ImageCopyWith<NoteSpan_Image> get copyWith => _$NoteSpan_ImageCopyWithImpl<NoteSpan_Image>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is NoteSpan_Image&&(identical(other.image, image) || other.image == image));
}


@override
int get hashCode {
    return Object.hash(runtimeType,image);
}

@override
String toString() {
    return 'NoteSpan.image(image: $image)';
}


}

/// @nodoc
abstract mixin class $NoteSpan_ImageCopyWith<$Res> implements $NoteSpanCopyWith<$Res> {
  factory $NoteSpan_ImageCopyWith(NoteSpan_Image value, $Res Function(NoteSpan_Image) _then) = _$NoteSpan_ImageCopyWithImpl;
@useResult
$Res call({
 ImageRef image
});




}
/// @nodoc
class _$NoteSpan_ImageCopyWithImpl<$Res>
    implements $NoteSpan_ImageCopyWith<$Res> {
  _$NoteSpan_ImageCopyWithImpl(this._self, this._then);

  final NoteSpan_Image _self;
  final $Res Function(NoteSpan_Image) _then;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? image = null,}) {
  return _then(NoteSpan_Image(
image: null == image ? _self.image : image // ignore: cast_nullable_to_non_nullable
as ImageRef,
  ));
}


}

/// @nodoc


class NoteSpan_Embed extends NoteSpan {
  const NoteSpan_Embed({required this.url, this.title, required this.kind}): super._();
  

 final  String url;
 final  String? title;
 final  EmbedKind kind;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$NoteSpan_EmbedCopyWith<NoteSpan_Embed> get copyWith => _$NoteSpan_EmbedCopyWithImpl<NoteSpan_Embed>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is NoteSpan_Embed&&(identical(other.url, url) || other.url == url)&&(identical(other.title, title) || other.title == title)&&(identical(other.kind, kind) || other.kind == kind));
}


@override
int get hashCode {
    return Object.hash(runtimeType,url,title,kind);
}

@override
String toString() {
    return 'NoteSpan.embed(url: $url, title: $title, kind: $kind)';
}


}

/// @nodoc
abstract mixin class $NoteSpan_EmbedCopyWith<$Res> implements $NoteSpanCopyWith<$Res> {
  factory $NoteSpan_EmbedCopyWith(NoteSpan_Embed value, $Res Function(NoteSpan_Embed) _then) = _$NoteSpan_EmbedCopyWithImpl;
@useResult
$Res call({
 String url, String? title, EmbedKind kind
});


$EmbedKindCopyWith<$Res> get kind;

}
/// @nodoc
class _$NoteSpan_EmbedCopyWithImpl<$Res>
    implements $NoteSpan_EmbedCopyWith<$Res> {
  _$NoteSpan_EmbedCopyWithImpl(this._self, this._then);

  final NoteSpan_Embed _self;
  final $Res Function(NoteSpan_Embed) _then;

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? url = null,Object? title = freezed,Object? kind = null,}) {
  return _then(NoteSpan_Embed(
url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,title: freezed == title ? _self.title : title // ignore: cast_nullable_to_non_nullable
as String?,kind: null == kind ? _self.kind : kind // ignore: cast_nullable_to_non_nullable
as EmbedKind,
  ));
}

/// Create a copy of NoteSpan
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$EmbedKindCopyWith<$Res> get kind {
  
  return $EmbedKindCopyWith<$Res>(_self.kind, (value) {
    return _then(_self.copyWith(kind: value));
  });
}
}

// dart format on
